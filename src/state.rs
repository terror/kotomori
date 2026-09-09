use super::*;

#[derive(Debug)]
pub(crate) struct State {
  pub(crate) composer: Composer,
  database: Database,
  pub(crate) directory: PathBuf,
  pub(crate) model: Model,
  next_run_id: u64,
  queued_inputs: VecDeque<String>,
  run: Option<Run>,
  pub(crate) session: Session,
  pub(crate) should_quit: bool,
}

impl State {
  pub(crate) fn active_run(&self) -> Option<&Run> {
    self.run.as_ref()
  }

  pub(crate) fn approval(&self) -> Option<&ApprovalRequest> {
    self.run.as_ref().and_then(|run| run.approval.as_ref())
  }

  fn finish_run(&mut self, entry: Option<TranscriptEntry>) {
    if let Some(message) = self.run.take().and_then(Run::finish) {
      self.session.transcript.push_message(message);
    }

    self.session.transcript.entries.extend(entry);

    self.save_session();
  }

  fn handle_action(&mut self, action: Action) -> Vec<Effect> {
    if action == Action::Quit && self.run.is_some() {
      return self.interrupt_agent();
    }

    match self.approval() {
      Some(_) => match action {
        Action::Edit(input) if input.key == Key::Char('y') => {
          self.resolve_approval(ToolApproval::Approved);
        }
        Action::Edit(input) if input.key == Key::Char('Y') => {
          self.resolve_approval(ToolApproval::Approved);
        }
        Action::Edit(input) if input.key == Key::Char('n') => {
          self.resolve_approval(ToolApproval::Denied);
        }
        Action::Edit(input) if input.key == Key::Char('N') => {
          self.resolve_approval(ToolApproval::Denied);
        }
        Action::Interrupt => {
          self.resolve_approval(ToolApproval::Denied);
        }
        Action::CompleteCommand
        | Action::Edit(_)
        | Action::Quit
        | Action::SelectNext
        | Action::SelectPrevious
        | Action::Submit
        | Action::SubmitImmediately => {}
      },
      None => match action {
        Action::CompleteCommand => {
          if !self.composer.complete_command() {
            self.composer.input(Input {
              key: Key::Tab,
              ..Default::default()
            });
          }
        }
        Action::Edit(input) => self.composer.input(input),
        Action::Interrupt => {
          let effects = self.interrupt_agent();

          if effects.is_empty() {
            return effects;
          }

          return effects.into_iter().chain(self.run_next_queued()).collect();
        }
        Action::Quit => self.quit(),
        Action::SelectNext => self.composer.select_next(),
        Action::SelectPrevious => self.composer.select_previous(),
        Action::Submit | Action::SubmitImmediately => {
          return self.submit(&action);
        }
      },
    }

    Vec::new()
  }

  fn handle_command(&mut self, command: Command) -> Vec<Effect> {
    let effects = match command {
      Command::Clear => {
        let interrupt_agent = self.run.take().is_some();

        self.session.transcript.clear();
        self.composer.clear_history();
        self.queued_inputs.clear();

        self.save_session();

        if interrupt_agent {
          vec![Effect::InterruptAgent]
        } else {
          Vec::new()
        }
      }
      Command::Quit => self.handle_action(Action::Quit),
    };

    self.reset_input();

    effects
  }

  pub(crate) fn handle_event(&mut self, event: Event) -> Vec<Effect> {
    match event {
      Event::Action(action) => return self.handle_action(action),
      Event::Agent { event, run_id } => {
        let Some(run) = self.run.as_mut().filter(|run| run.id == run_id) else {
          return Vec::new();
        };

        match event {
          AgentEvent::Done => {
            self.finish_run(None);
            return self.run_next_queued();
          }
          AgentEvent::Delta(delta) => {
            run.push_delta(&delta);
          }
          AgentEvent::ReasoningDelta(delta) => {
            run.push_reasoning_delta(&delta);
          }
          AgentEvent::Message(message) => {
            run.reset_message();
            self.session.transcript.push_message(message);
            self.save_session();
          }
          AgentEvent::Error(error) => {
            self.finish_run(Some(TranscriptEntry::Error(error)));
            return self.run_next_queued();
          }
          AgentEvent::ToolApprovalRequest(request) => {
            run.approval = Some(request);
          }
        }
      }
      Event::Error(error) => {
        let effects = if self.run.is_some() {
          vec![Effect::InterruptAgent]
        } else {
          Vec::new()
        };

        self.finish_run(Some(TranscriptEntry::Error(error)));

        return effects;
      }
      Event::Tick(elapsed) => {
        if let Some(run) = &mut self.run {
          run.tick(elapsed);
        }
      }
    }

    Vec::new()
  }

  fn interrupt_agent(&mut self) -> Vec<Effect> {
    if self.run.is_none() {
      return Vec::new();
    }

    self.finish_run(Some(TranscriptEntry::Interrupted));

    vec![Effect::InterruptAgent]
  }

  pub(crate) fn new(settings: &Settings) -> Result<Self> {
    Self::with_session(settings, Database::new()?, Session::new(settings)?)
  }

  pub(crate) fn queued_inputs(&self) -> &VecDeque<String> {
    &self.queued_inputs
  }

  fn quit(&mut self) {
    self.should_quit = true;
  }

  fn reset_input(&mut self) {
    self.composer.clear();
  }

  fn resolve_approval(&mut self, approval: ToolApproval) {
    if let Some(run) = &mut self.run {
      run.resolve_approval(approval);
    }
  }

  fn run(&mut self, input: String) -> Effect {
    self.session.transcript.send(input);

    self.save_session();

    let messages = self.session.transcript.messages();

    let run_id = self.next_run_id;

    self.next_run_id = self
      .next_run_id
      .checked_add(1)
      .expect("agent run ID overflow");

    self.run = Some(Run::new(run_id));

    Effect::RunAgent { messages, run_id }
  }

  fn run_next_queued(&mut self) -> Vec<Effect> {
    self
      .queued_inputs
      .pop_front()
      .map(|input| vec![self.run(input)])
      .unwrap_or_default()
  }

  fn save_session(&mut self) {
    if let Err(error) = self.session.save(&self.database) {
      self
        .session
        .transcript
        .error(format!("failed to save session: {error}"));
    }
  }

  fn submit(&mut self, action: &Action) -> Vec<Effect> {
    let input = self.composer.input_text();
    let input = input.trim();

    if let Some(command) =
      Command::from_input(input).or_else(|| self.composer.selected_command())
    {
      return self.handle_command(command);
    }

    if input.starts_with('/') {
      if input.len() > 1 {
        self.session.transcript.notice(format!(
          "Unrecognized command '{input}'. Type \"/\" for a list of supported commands."
        ));

        self.reset_input();
      }

      return Vec::new();
    }

    if input.is_empty() {
      return Vec::new();
    }

    let input = input.to_string();

    self.composer.remember(&input);
    self.reset_input();

    match action {
      Action::SubmitImmediately => self
        .interrupt_agent()
        .into_iter()
        .chain(once(self.run(input)))
        .collect(),
      _ if self.run.is_some() => {
        self.queued_inputs.push_back(input);
        Vec::new()
      }
      _ => vec![self.run(input)],
    }
  }

  pub(crate) fn with_session(
    settings: &Settings,
    database: Database,
    mut session: Session,
  ) -> Result<Self> {
    let history = session
      .transcript
      .entries
      .iter()
      .filter_map(TranscriptEntry::message)
      .filter_map(Message::user_content)
      .map(str::to_owned)
      .collect();

    session.set_model(&settings.model);

    Ok(Self {
      composer: Composer::new(
        settings.prompt.as_deref().unwrap_or_default(),
        history,
      ),
      database,
      directory: env::current_dir()?,
      model: settings.model.clone(),
      next_run_id: 0,
      queued_inputs: VecDeque::new(),
      run: None,
      session,
      should_quit: false,
    })
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn agent_events_update_transcript() {
    let mut state = State::new(&Settings {
      model: "mock:local".parse().unwrap(),
      prompt: Some("foo".into()),
      yolo: false,
    })
    .unwrap();

    assert_eq!(
      state.handle_event(Event::Action(Action::Submit)),
      vec![Effect::RunAgent {
        messages: vec![Message::User(vec![UserMessageContent::Text(
          "foo".into()
        )])],
        run_id: 0,
      }]
    );

    state.handle_event(Event::Agent {
      event: AgentEvent::ReasoningDelta("bar".into()),
      run_id: 0,
    });
    state.handle_event(Event::Agent {
      event: AgentEvent::Delta("baz".into()),
      run_id: 0,
    });

    let invocation = ToolInvocation {
      id: "foo".into(),
      kind: ToolInvocationKind::Command(CommandTool {
        command: "bar".into(),
        cwd: None,
      }),
    };

    state.handle_event(Event::Agent {
      event: AgentEvent::Message(Message::Agent(vec![
        AgentMessageContent::Reasoning("bar".into()),
        AgentMessageContent::Text("baz".into()),
        AgentMessageContent::ToolCall(invocation.clone()),
      ])),
      run_id: 0,
    });

    let result = ToolResult {
      exit_status: Some(0),
      outcome: ToolOutcome::Success,
      stdout: Some("qux".into()),
      ..Default::default()
    };

    state.handle_event(Event::Agent {
      event: AgentEvent::Message(Message::User(vec![
        UserMessageContent::ToolResult {
          id: "foo".into(),
          result: result.clone(),
        },
      ])),
      run_id: 0,
    });

    state.handle_event(Event::Agent {
      event: AgentEvent::Done,
      run_id: 0,
    });

    assert_eq!(state.run, None);

    assert_eq!(
      state.session.transcript.messages(),
      vec![
        Message::User(vec![UserMessageContent::Text("foo".into())]),
        Message::Agent(vec![
          AgentMessageContent::Reasoning("bar".into()),
          AgentMessageContent::Text("baz".into()),
          AgentMessageContent::ToolCall(invocation),
        ]),
        Message::User(vec![UserMessageContent::ToolResult {
          id: "foo".into(),
          result,
        }]),
      ],
    );
  }

  #[test]
  fn approval_inputs_leave_request_pending() {
    #[track_caller]
    fn case(action: Action) {
      let mut state = State::new(&Settings {
        model: "mock:local".parse().unwrap(),
        prompt: Some("foo".into()),
        yolo: false,
      })
      .unwrap();

      let (request, mut response_receiver) =
        ApprovalRequest::new(ToolInvocation {
          id: "foo".into(),
          kind: ToolInvocationKind::Command(CommandTool {
            command: "bar".into(),
            cwd: None,
          }),
        });

      let invocation = request.invocation.clone();

      state.run = Some(Run {
        approval: Some(request),
        ..Run::new(0)
      });

      assert_eq!(state.handle_event(Event::Action(action)), Vec::new());
      assert_eq!(state.approval().unwrap().invocation, invocation);
      assert_eq!(state.composer.input_text(), "foo");
      assert_eq!(
        response_receiver.try_recv(),
        Err(oneshot::error::TryRecvError::Empty)
      );
    }

    case(Action::CompleteCommand);
    case(Action::Edit(Input {
      key: Key::Char('x'),
      ..Default::default()
    }));
    case(Action::SelectNext);
    case(Action::SelectPrevious);
    case(Action::Submit);
    case(Action::SubmitImmediately);
  }

  #[test]
  fn approval_inputs_resolve_request() {
    #[track_caller]
    fn case(action: Action, approval: ToolApproval) {
      let mut state = State::new(&Settings {
        model: "mock:local".parse().unwrap(),
        prompt: Some("foo".into()),
        yolo: false,
      })
      .unwrap();

      let (request, mut response_receiver) =
        ApprovalRequest::new(ToolInvocation {
          id: "foo".into(),
          kind: ToolInvocationKind::Command(CommandTool {
            command: "bar".into(),
            cwd: None,
          }),
        });

      state.run = Some(Run {
        approval: Some(request),
        ..Run::new(0)
      });

      assert_eq!(state.handle_event(Event::Action(action)), Vec::new());
      assert_eq!(response_receiver.try_recv().unwrap(), approval);
      assert_eq!(state.run, Some(Run::new(0)));
      assert_eq!(state.composer.input_text(), "foo");
    }

    for (key, approval) in [
      ('y', ToolApproval::Approved),
      ('Y', ToolApproval::Approved),
      ('n', ToolApproval::Denied),
      ('N', ToolApproval::Denied),
    ] {
      case(
        Action::Edit(Input {
          key: Key::Char(key),
          ..Default::default()
        }),
        approval,
      );
    }

    case(Action::Interrupt, ToolApproval::Denied);
  }

  #[tokio::test]
  async fn approval_terminal_agent_tool_result_drops_pending_request() {
    let mut state = State::new(&Settings {
      model: "mock:local".parse().unwrap(),
      prompt: Some(String::new()),
      yolo: false,
    })
    .unwrap();

    let (request, response_receiver) = ApprovalRequest::new(ToolInvocation {
      id: "foo".into(),
      kind: ToolInvocationKind::Command(CommandTool {
        command: "bar".into(),
        cwd: None,
      }),
    });

    state.run = Some(Run {
      approval: Some(request),
      ..Run::new(0)
    });

    state.handle_event(Event::Agent {
      event: AgentEvent::Message(Message::User(vec![
        UserMessageContent::ToolResult {
          id: "foo".into(),
          result: ToolResult {
            content: Some("bar".into()),
            ..Default::default()
          },
        },
      ])),
      run_id: 0,
    });

    assert_eq!(state.run, Some(Run::new(0)));
    assert!(response_receiver.await.is_err());
  }

  #[tokio::test]
  async fn approval_terminal_error_drops_pending_request() {
    let mut state = State::new(&Settings {
      model: "mock:local".parse().unwrap(),
      prompt: None,
      yolo: false,
    })
    .unwrap();

    let (request, response_receiver) = ApprovalRequest::new(ToolInvocation {
      id: "foo".into(),
      kind: ToolInvocationKind::Command(CommandTool {
        command: "bar".into(),
        cwd: None,
      }),
    });

    state.run = Some(Run {
      approval: Some(request),
      ..Run::new(0)
    });

    state.handle_event(Event::Agent {
      event: AgentEvent::Error("bar".into()),
      run_id: 0,
    });

    assert_eq!(state.run, None);
    assert!(response_receiver.await.is_err());
  }

  #[test]
  fn blank_submit_does_nothing() {
    let mut state = State::new(&Settings {
      model: "mock:local".parse().unwrap(),
      prompt: Some("  ".into()),
      yolo: false,
    })
    .unwrap();

    assert_eq!(
      state.handle_event(Event::Action(Action::Submit)),
      Vec::new()
    );

    assert!(state.session.transcript.messages().is_empty());

    assert_eq!(state.composer.input_text(), "  ");
  }

  #[test]
  fn command_autocomplete_select_next() {
    let mut state = State::new(&Settings {
      model: "mock:local".parse().unwrap(),
      prompt: Some("/".into()),
      yolo: false,
    })
    .unwrap();

    assert_eq!(
      state
        .composer
        .commands()
        .map(Command::name)
        .collect::<Vec<_>>(),
      vec!["clear", "quit"],
    );

    state.handle_event(Event::Action(Action::SelectNext));
    state.handle_event(Event::Action(Action::CompleteCommand));

    assert_eq!(state.composer.input_text(), "/quit");
  }

  #[test]
  fn command_autocomplete_select_previous() {
    let mut state = State::new(&Settings {
      model: "mock:local".parse().unwrap(),
      prompt: Some("/".into()),
      yolo: false,
    })
    .unwrap();

    assert_eq!(
      state
        .composer
        .commands()
        .map(Command::name)
        .collect::<Vec<_>>(),
      vec!["clear", "quit"],
    );

    state.handle_event(Event::Action(Action::SelectPrevious));
    state.handle_event(Event::Action(Action::CompleteCommand));

    assert_eq!(state.composer.input_text(), "/quit");
  }

  #[test]
  fn command_clear_from_empty_slash() {
    let mut state = State::new(&Settings {
      model: "mock:local".parse().unwrap(),
      prompt: Some("foo".into()),
      yolo: false,
    })
    .unwrap();

    assert_eq!(
      state.handle_event(Event::Action(Action::Submit)),
      vec![Effect::RunAgent {
        messages: vec![Message::User(vec![UserMessageContent::Text(
          "foo".into()
        )])],
        run_id: 0,
      }]
    );

    state.handle_event(Event::Agent {
      event: AgentEvent::Delta("bar".into()),
      run_id: 0,
    });
    state.handle_event(Event::Agent {
      event: AgentEvent::Done,
      run_id: 0,
    });

    state.handle_event(Event::Action(Action::Edit(Input {
      key: Key::Char('/'),
      ..Default::default()
    })));

    assert_eq!(
      state.handle_event(Event::Action(Action::Submit)),
      Vec::new()
    );

    assert!(state.session.transcript.messages().is_empty());

    assert_eq!(state.composer.input_text(), "");
  }

  #[test]
  fn command_clear_from_name() {
    let mut state = State::new(&Settings {
      model: "mock:local".parse().unwrap(),
      prompt: Some("foo".into()),
      yolo: false,
    })
    .unwrap();

    assert_eq!(
      state.handle_event(Event::Action(Action::Submit)),
      vec![Effect::RunAgent {
        messages: vec![Message::User(vec![UserMessageContent::Text(
          "foo".into()
        )])],
        run_id: 0,
      }]
    );

    state.handle_event(Event::Agent {
      event: AgentEvent::Delta("bar".into()),
      run_id: 0,
    });
    state.handle_event(Event::Agent {
      event: AgentEvent::Done,
      run_id: 0,
    });

    for c in "/clear".chars() {
      state.handle_event(Event::Action(Action::Edit(Input {
        key: Key::Char(c),
        ..Default::default()
      })));
    }

    assert_eq!(
      state.handle_event(Event::Action(Action::Submit)),
      Vec::new()
    );

    assert!(state.session.transcript.messages().is_empty());

    assert_eq!(state.composer.input_text(), "");
  }

  #[test]
  fn command_clear_interrupts_active_agent_and_ignores_late_events() {
    let mut state = State::new(&Settings {
      model: "mock:local".parse().unwrap(),
      prompt: Some("foo".into()),
      yolo: false,
    })
    .unwrap();

    state.handle_event(Event::Action(Action::Submit));

    for c in "/clear".chars() {
      state.handle_event(Event::Action(Action::Edit(Input {
        key: Key::Char(c),
        ..Default::default()
      })));
    }

    assert_eq!(
      state.handle_event(Event::Action(Action::Submit)),
      vec![Effect::InterruptAgent]
    );

    assert_eq!(state.run, None);

    assert!(state.session.transcript.messages().is_empty());

    let invocation = ToolInvocation {
      id: "late".into(),
      kind: ToolInvocationKind::Command(CommandTool {
        command: "echo late".into(),
        cwd: None,
      }),
    };

    state.handle_event(Event::Agent {
      event: AgentEvent::Message(Message::Agent(vec![
        AgentMessageContent::ToolCall(invocation),
      ])),
      run_id: 0,
    });

    state.handle_event(Event::Agent {
      event: AgentEvent::Message(Message::User(vec![
        UserMessageContent::ToolResult {
          id: "late".into(),
          result: ToolResult::default(),
        },
      ])),
      run_id: 0,
    });

    state.handle_event(Event::Agent {
      event: AgentEvent::Done,
      run_id: 0,
    });

    assert!(state.session.transcript.messages().is_empty());
  }

  #[test]
  fn command_clear_from_prefix() {
    let mut state = State::new(&Settings {
      model: "mock:local".parse().unwrap(),
      prompt: Some("foo".into()),
      yolo: false,
    })
    .unwrap();

    assert_eq!(
      state.handle_event(Event::Action(Action::Submit)),
      vec![Effect::RunAgent {
        messages: vec![Message::User(vec![UserMessageContent::Text(
          "foo".into()
        )])],
        run_id: 0,
      }]
    );

    state.handle_event(Event::Agent {
      event: AgentEvent::Delta("bar".into()),
      run_id: 0,
    });
    state.handle_event(Event::Agent {
      event: AgentEvent::Done,
      run_id: 0,
    });

    for c in "/c".chars() {
      state.handle_event(Event::Action(Action::Edit(Input {
        key: Key::Char(c),
        ..Default::default()
      })));
    }

    assert_eq!(
      state.handle_event(Event::Action(Action::Submit)),
      Vec::new()
    );

    assert!(state.session.transcript.messages().is_empty());

    assert_eq!(state.composer.input_text(), "");
  }

  #[test]
  fn command_quit_from_name() {
    let mut state = State::new(&Settings {
      model: "mock:local".parse().unwrap(),
      prompt: Some("/quit".into()),
      yolo: false,
    })
    .unwrap();

    assert_eq!(
      state.handle_event(Event::Action(Action::Submit)),
      Vec::new()
    );

    assert!(state.should_quit);

    assert_eq!(state.composer.input_text(), "");
  }

  #[test]
  fn command_quit_from_prefix() {
    let mut state = State::new(&Settings {
      model: "mock:local".parse().unwrap(),
      prompt: Some("/q".into()),
      yolo: false,
    })
    .unwrap();

    assert_eq!(
      state.handle_event(Event::Action(Action::Submit)),
      Vec::new()
    );

    assert!(state.should_quit);

    assert_eq!(state.composer.input_text(), "");
  }

  #[test]
  fn command_quit_interrupts_active_agent_and_saves_partial_output() {
    let mut state = State::new(&Settings {
      model: "mock:local".parse().unwrap(),
      prompt: Some("foo".into()),
      yolo: false,
    })
    .unwrap();

    state.handle_event(Event::Action(Action::Submit));

    state.handle_event(Event::Agent {
      event: AgentEvent::Delta("partial response".into()),
      run_id: 0,
    });

    for character in "/quit".chars() {
      state.handle_event(Event::Action(Action::Edit(Input {
        key: Key::Char(character),
        ..Default::default()
      })));
    }

    assert_eq!(
      state.handle_event(Event::Action(Action::Submit)),
      vec![Effect::InterruptAgent]
    );

    assert!(!state.should_quit);
    assert_eq!(state.run, None);

    assert_eq!(state.composer.input_text(), "");

    let saved = state
      .database
      .load_session(state.session.id.unwrap())
      .unwrap();

    assert_eq!(
      saved.transcript.entries,
      [
        TranscriptEntry::Message(Message::User(vec![
          UserMessageContent::Text("foo".into())
        ])),
        TranscriptEntry::Message(Message::Agent(vec![
          AgentMessageContent::Text("partial response".into())
        ])),
        TranscriptEntry::Interrupted,
      ],
    );
  }

  #[test]
  fn command_submission_returns_effects() {
    #[track_caller]
    fn case(input: &str, action: Action) {
      let mut state = State::new(&Settings {
        model: "mock:local".parse().unwrap(),
        prompt: Some(input.into()),
        yolo: false,
      })
      .unwrap();

      state.run("foo".into());

      assert_eq!(
        state.handle_event(Event::Action(action)),
        vec![Effect::InterruptAgent]
      );

      assert_eq!(state.run, None);
      assert_eq!(state.composer.input_text(), "");
    }

    case("/cl", Action::Submit);
    case("/cl", Action::SubmitImmediately);
    case("/q", Action::Submit);
    case("/q", Action::SubmitImmediately);
    case("  /clear  ", Action::Submit);
  }

  #[test]
  fn completed_messages_survive_resumption() {
    let settings = Settings {
      model: "mock:local".parse().unwrap(),
      prompt: Some("foo".into()),
      yolo: false,
    };

    let mut state = State::new(&settings).unwrap();

    state.handle_event(Event::Action(Action::Submit));

    state.handle_event(Event::Agent {
      event: AgentEvent::Delta("bar".into()),
      run_id: 0,
    });

    state.handle_event(Event::Agent {
      event: AgentEvent::ReasoningDelta("baz".into()),
      run_id: 0,
    });

    let messages = vec![
      Message::Agent(vec![
        AgentMessageContent::Reasoning("foo".into()),
        AgentMessageContent::Text("bar".into()),
        AgentMessageContent::ToolCall(ToolInvocation {
          id: "foo".into(),
          kind: ToolInvocationKind::Command(CommandTool {
            command: "bar".into(),
            cwd: None,
          }),
        }),
        AgentMessageContent::Text("baz".into()),
        AgentMessageContent::ToolCall(ToolInvocation {
          id: "bar".into(),
          kind: ToolInvocationKind::Command(CommandTool {
            command: "qux".into(),
            cwd: None,
          }),
        }),
      ]),
      Message::User(vec![
        UserMessageContent::ToolResult {
          id: "bar".into(),
          result: ToolResult {
            content: Some("foo".into()),
            ..Default::default()
          },
        },
        UserMessageContent::ToolResult {
          id: "foo".into(),
          result: ToolResult {
            content: Some("bar".into()),
            ..Default::default()
          },
        },
      ]),
      Message::Agent(vec![AgentMessageContent::Text("qux".into())]),
    ];

    for message in &messages {
      state.handle_event(Event::Agent {
        event: AgentEvent::Message(message.clone()),
        run_id: 0,
      });
    }

    state.handle_event(Event::Agent {
      event: AgentEvent::Done,
      run_id: 0,
    });

    let session = state
      .database
      .load_session(state.session.id.unwrap())
      .unwrap();

    assert_eq!(session.transcript.entries, state.session.transcript.entries);
    assert_eq!(session.title.as_deref(), Some("foo"));

    let mut state =
      State::with_session(&settings, state.database, session).unwrap();

    let messages =
      once(Message::User(vec![UserMessageContent::Text("foo".into())]))
        .chain(messages)
        .chain(once(Message::User(vec![UserMessageContent::Text(
          "foo".into(),
        )])))
        .collect();

    assert_eq!(
      state.handle_event(Event::Action(Action::Submit)),
      [Effect::RunAgent {
        messages,
        run_id: 0
      }]
    );
  }

  #[test]
  fn error_is_not_included_in_next_request() {
    let mut state = State::new(&Settings {
      model: "mock:local".parse().unwrap(),
      prompt: Some("foo".into()),
      yolo: false,
    })
    .unwrap();

    assert_eq!(
      state.handle_event(Event::Action(Action::Submit)),
      vec![Effect::RunAgent {
        messages: vec![Message::User(vec![UserMessageContent::Text(
          "foo".into()
        )])],
        run_id: 0,
      }]
    );

    state.handle_event(Event::Agent {
      event: AgentEvent::Error("bar".into()),
      run_id: 0,
    });

    assert_eq!(
      state.session.transcript.messages(),
      [Message::User(vec![UserMessageContent::Text("foo".into())])]
    );

    for c in "baz".chars() {
      state.handle_event(Event::Action(Action::Edit(Input {
        key: Key::Char(c),
        ..Default::default()
      })));
    }

    assert_eq!(
      state.handle_event(Event::Action(Action::Submit)),
      vec![Effect::RunAgent {
        messages: vec![
          Message::User(vec![UserMessageContent::Text("foo".into())]),
          Message::User(vec![UserMessageContent::Text("baz".into())]),
        ],
        run_id: 1,
      }]
    );
  }

  #[test]
  fn failed_save_preserves_run() {
    let mut state = State::new(&Settings {
      model: "mock:local".parse().unwrap(),
      prompt: None,
      yolo: false,
    })
    .unwrap();

    state.run = Some(Run {
      activity: AgentActivity::Streaming("foo".into()),
      ..Run::new(0)
    });

    state.session.id = Some(0);

    state.save_session();

    assert_eq!(
      state.run,
      Some(Run {
        activity: AgentActivity::Streaming("foo".into()),
        ..Run::new(0)
      })
    );
    assert_eq!(
      state.session.transcript.entries,
      [TranscriptEntry::Error(
        "failed to save session: session `0` no longer exists".into()
      )]
    );
  }

  #[test]
  fn finish_run_saves_partial_output() {
    let mut state = State::new(&Settings {
      model: "mock:local".parse().unwrap(),
      prompt: None,
      yolo: false,
    })
    .unwrap();

    state.run = Some(Run {
      activity: AgentActivity::Streaming("foo".into()),
      ..Run::new(0)
    });

    state.finish_run(None);

    let session = state
      .database
      .load_session(state.session.id.unwrap())
      .unwrap();

    assert_eq!(state.run, None);
    assert_eq!(
      state.session.transcript.messages(),
      [Message::Agent(vec![AgentMessageContent::Text(
        "foo".into()
      )])]
    );
    assert_eq!(session.transcript, state.session.transcript);
  }

  #[test]
  fn immediate_submit_interrupts_active_agent_and_starts_new_run() {
    let mut state = State::new(&Settings {
      model: "mock:local".parse().unwrap(),
      prompt: Some("foo".into()),
      yolo: false,
    })
    .unwrap();

    state.handle_event(Event::Action(Action::Submit));

    state.handle_event(Event::Agent {
      event: AgentEvent::Delta("partial".into()),
      run_id: 0,
    });

    state.queued_inputs.push_back("baz".into());

    for c in "  bar  ".chars() {
      state.handle_event(Event::Action(Action::Edit(Input {
        key: Key::Char(c),
        ..Default::default()
      })));
    }

    assert_eq!(
      state.handle_event(Event::Action(Action::SubmitImmediately)),
      vec![
        Effect::InterruptAgent,
        Effect::RunAgent {
          messages: vec![
            Message::User(vec![UserMessageContent::Text("foo".into())]),
            Message::Agent(vec![AgentMessageContent::Text("partial".into())]),
            Message::User(vec![UserMessageContent::Text("bar".into())]),
          ],
          run_id: 1,
        },
      ]
    );

    assert_eq!(state.run, Some(Run::new(1)));

    assert_eq!(state.composer.input_text(), "");
    assert_eq!(state.queued_inputs(), &VecDeque::from(["baz".into()]));

    state.handle_event(Event::Action(Action::SelectPrevious));

    assert_eq!(state.composer.input_text(), "bar");

    state.handle_event(Event::Action(Action::SelectPrevious));

    assert_eq!(state.composer.input_text(), "foo");

    state.handle_event(Event::Agent {
      event: AgentEvent::Done,
      run_id: 0,
    });

    assert_eq!(state.run, Some(Run::new(1)));
  }

  #[test]
  fn interrupt_advances_to_next_queued_submission() {
    let mut state = State::new(&Settings {
      model: "mock:local".parse().unwrap(),
      prompt: Some("first".into()),
      yolo: false,
    })
    .unwrap();

    state.handle_event(Event::Action(Action::Submit));

    for c in "second".chars() {
      state.handle_event(Event::Action(Action::Edit(Input {
        key: Key::Char(c),
        ..Default::default()
      })));
    }

    state.handle_event(Event::Action(Action::Submit));

    assert_matches!(
      state
        .handle_event(Event::Action(Action::Interrupt))
        .as_slice(),
      [Effect::InterruptAgent, Effect::RunAgent { run_id: 1, .. }]
    );

    assert!(state.queued_inputs().is_empty());
    assert_eq!(state.run, Some(Run::new(1)));
  }

  #[test]
  fn interrupt_stops_active_agent() {
    let mut state = State::new(&Settings {
      model: "mock:local".parse().unwrap(),
      prompt: Some("foo".into()),
      yolo: false,
    })
    .unwrap();

    assert_eq!(
      state.handle_event(Event::Action(Action::Submit)),
      vec![Effect::RunAgent {
        messages: vec![Message::User(vec![UserMessageContent::Text(
          "foo".into()
        )])],
        run_id: 0,
      }]
    );

    assert_eq!(
      state.handle_event(Event::Action(Action::Interrupt)),
      vec![Effect::InterruptAgent]
    );

    assert_eq!(state.run, None);

    assert_eq!(
      state.handle_event(Event::Action(Action::Interrupt)),
      Vec::new()
    );
  }

  #[test]
  fn multiline_input() {
    let mut state = State::new(&Settings {
      model: "mock:local".parse().unwrap(),
      prompt: Some(String::new()),
      yolo: false,
    })
    .unwrap();

    for c in "foo".chars() {
      state.handle_event(Event::Action(Action::Edit(Input {
        key: Key::Char(c),
        ..Default::default()
      })));
    }

    state.handle_event(Event::Action(Action::Edit(Input {
      key: Key::Enter,
      ..Default::default()
    })));

    for c in "bar".chars() {
      state.handle_event(Event::Action(Action::Edit(Input {
        key: Key::Char(c),
        ..Default::default()
      })));
    }

    assert_eq!(
      state.handle_event(Event::Action(Action::Submit)),
      vec![Effect::RunAgent {
        messages: vec![Message::User(vec![UserMessageContent::Text(
          "foo\nbar".into()
        )])],
        run_id: 0,
      }]
    );
  }

  #[test]
  fn new_uses_empty_prompt_by_default() {
    let state = State::new(&Settings {
      model: "mock:local".parse().unwrap(),
      prompt: None,
      yolo: false,
    })
    .unwrap();

    assert_eq!(state.composer.input_text(), "");
  }

  #[test]
  fn prompt_history_edit_detaches_navigation() {
    let mut state = State::new(&Settings {
      model: "mock:local".parse().unwrap(),
      prompt: Some("history".into()),
      yolo: false,
    })
    .unwrap();

    state.handle_event(Event::Action(Action::Submit));

    state.handle_event(Event::Agent {
      event: AgentEvent::Done,
      run_id: 0,
    });

    state.handle_event(Event::Action(Action::SelectPrevious));

    state.handle_event(Event::Action(Action::Edit(Input {
      key: Key::Char('!'),
      ..Default::default()
    })));

    state.handle_event(Event::Action(Action::SelectNext));
    assert_eq!(state.composer.input_text(), "history!");

    state.handle_event(Event::Action(Action::SelectPrevious));
    assert_eq!(state.composer.input_text(), "history");

    state.handle_event(Event::Action(Action::SelectNext));
    assert_eq!(state.composer.input_text(), "history!");
  }

  #[test]
  fn prompt_history_is_cleared_by_clear_command() {
    let mut state = State::new(&Settings {
      model: "mock:local".parse().unwrap(),
      prompt: Some("history".into()),
      yolo: false,
    })
    .unwrap();

    state.handle_event(Event::Action(Action::Submit));

    state.handle_event(Event::Agent {
      event: AgentEvent::Done,
      run_id: 0,
    });

    for c in "/clear".chars() {
      state.handle_event(Event::Action(Action::Edit(Input {
        key: Key::Char(c),
        ..Default::default()
      })));
    }

    state.handle_event(Event::Action(Action::Submit));
    state.handle_event(Event::Action(Action::SelectPrevious));

    assert_eq!(state.composer.input_text(), "");
  }

  #[test]
  fn prompt_history_loads_session() {
    let settings = Settings {
      model: "mock:local".parse().unwrap(),
      prompt: Some("draft".into()),
      yolo: false,
    };

    let mut session = Session::new(&settings).unwrap();

    session.transcript.entries = vec![
      TranscriptEntry::Message(Message::User(vec![UserMessageContent::Text(
        "foo".into(),
      )])),
      TranscriptEntry::Message(Message::Agent(vec![
        AgentMessageContent::Text("bar".into()),
      ])),
      TranscriptEntry::Message(Message::User(vec![UserMessageContent::Text(
        "baz\nqux".into(),
      )])),
    ];

    let mut state =
      State::with_session(&settings, Database::new().unwrap(), session)
        .unwrap();

    state.handle_event(Event::Action(Action::SelectPrevious));
    assert_eq!(state.composer.input_text(), "baz\nqux");

    state.handle_event(Event::Action(Action::SelectPrevious));
    assert_eq!(state.composer.cursor().0, 0);

    state.handle_event(Event::Action(Action::SelectPrevious));
    assert_eq!(state.composer.input_text(), "foo");
  }

  #[test]
  fn prompt_history_navigates_and_restores_draft() {
    let mut state = State::new(&Settings {
      model: "mock:local".parse().unwrap(),
      prompt: Some("foo".into()),
      yolo: false,
    })
    .unwrap();

    state.handle_event(Event::Action(Action::Submit));

    state.handle_event(Event::Agent {
      event: AgentEvent::Done,
      run_id: 0,
    });

    for c in "bar".chars() {
      state.handle_event(Event::Action(Action::Edit(Input {
        key: Key::Char(c),
        ..Default::default()
      })));
    }

    state.handle_event(Event::Action(Action::Submit));
    state.handle_event(Event::Agent {
      event: AgentEvent::Done,
      run_id: 1,
    });

    for c in "draft".chars() {
      state.handle_event(Event::Action(Action::Edit(Input {
        key: Key::Char(c),
        ..Default::default()
      })));
    }

    state.handle_event(Event::Action(Action::SelectPrevious));
    assert_eq!(state.composer.input_text(), "bar");

    state.handle_event(Event::Action(Action::SelectPrevious));
    assert_eq!(state.composer.input_text(), "foo");

    state.handle_event(Event::Action(Action::SelectNext));
    assert_eq!(state.composer.input_text(), "bar");

    state.handle_event(Event::Action(Action::SelectNext));
    assert_eq!(state.composer.input_text(), "draft");
  }

  #[test]
  fn prompt_history_preserves_multiline_navigation() {
    let mut state = State::new(&Settings {
      model: "mock:local".parse().unwrap(),
      prompt: Some("history".into()),
      yolo: false,
    })
    .unwrap();

    state.handle_event(Event::Action(Action::Submit));

    state.handle_event(Event::Agent {
      event: AgentEvent::Done,
      run_id: 0,
    });

    for input in [
      Input {
        key: Key::Char('a'),
        ..Default::default()
      },
      Input {
        key: Key::Enter,
        ..Default::default()
      },
      Input {
        key: Key::Char('b'),
        ..Default::default()
      },
    ] {
      state.handle_event(Event::Action(Action::Edit(input)));
    }

    state.handle_event(Event::Action(Action::SelectPrevious));
    assert_eq!(state.composer.input_text(), "a\nb");
    assert_eq!(state.composer.cursor().0, 0);

    state.handle_event(Event::Action(Action::SelectPrevious));
    assert_eq!(state.composer.input_text(), "history");

    state.handle_event(Event::Action(Action::SelectNext));
    assert_eq!(state.composer.input_text(), "a\nb");
    assert_eq!(state.composer.cursor().0, 1);
  }

  #[test]
  fn queued_submissions_run_in_order() {
    let mut state = State::new(&Settings {
      model: "mock:local".parse().unwrap(),
      prompt: Some("first".into()),
      yolo: false,
    })
    .unwrap();

    state.handle_event(Event::Action(Action::Submit));

    for input in ["second", "third"] {
      for c in input.chars() {
        state.handle_event(Event::Action(Action::Edit(Input {
          key: Key::Char(c),
          ..Default::default()
        })));
      }

      state.handle_event(Event::Action(Action::Submit));
    }

    assert_eq!(state.queued_inputs().len(), 2);

    assert_matches!(
      state
        .handle_event(Event::Agent {
          event: AgentEvent::Done,
          run_id: 0,
        })
        .as_slice(),
      [Effect::RunAgent { run_id: 1, .. }]
    );

    assert_matches!(
      state
        .handle_event(Event::Agent {
          event: AgentEvent::Done,
          run_id: 1,
        })
        .as_slice(),
      [Effect::RunAgent { run_id: 2, .. }]
    );

    assert_eq!(
      state.session.transcript.messages(),
      [
        Message::User(vec![UserMessageContent::Text("first".into())]),
        Message::User(vec![UserMessageContent::Text("second".into())]),
        Message::User(vec![UserMessageContent::Text("third".into())]),
      ]
    );
  }

  #[test]
  fn quit_interrupts_active_agent() {
    let mut state = State::new(&Settings {
      model: "mock:local".parse().unwrap(),
      prompt: Some("foo".into()),
      yolo: false,
    })
    .unwrap();

    assert_eq!(
      state.handle_event(Event::Action(Action::Submit)),
      vec![Effect::RunAgent {
        messages: vec![Message::User(vec![UserMessageContent::Text(
          "foo".into()
        )])],
        run_id: 0,
      }]
    );

    assert_eq!(
      state.handle_event(Event::Action(Action::Quit)),
      vec![Effect::InterruptAgent]
    );

    assert!(!state.should_quit);
    assert_eq!(state.run, None);

    assert_eq!(state.handle_event(Event::Action(Action::Quit)), Vec::new());

    assert!(state.should_quit);
  }

  #[tokio::test]
  async fn quit_interrupts_active_approval() {
    let mut state = State::new(&Settings {
      model: "mock:local".parse().unwrap(),
      prompt: Some("foo".into()),
      yolo: false,
    })
    .unwrap();

    state.handle_event(Event::Action(Action::Submit));

    let (request, response_receiver) = ApprovalRequest::new(ToolInvocation {
      id: "foo".into(),
      kind: ToolInvocationKind::Command(CommandTool {
        command: "bar".into(),
        cwd: None,
      }),
    });

    state.handle_event(Event::Agent {
      event: AgentEvent::ToolApprovalRequest(request),
      run_id: 0,
    });

    assert!(state.approval().is_some());

    assert_eq!(
      state.handle_event(Event::Action(Action::Quit)),
      vec![Effect::InterruptAgent]
    );

    assert!(!state.should_quit);
    assert_eq!(state.run, None);

    assert_eq!(state.approval(), None);
    assert!(response_receiver.await.is_err());
  }

  #[test]
  fn save_excludes_streaming_content() {
    let mut state = State::new(&Settings {
      model: "mock:local".parse().unwrap(),
      prompt: None,
      yolo: false,
    })
    .unwrap();

    state.session.transcript.send("foo".into());

    state.run = Some(Run {
      activity: AgentActivity::Streaming("bar".into()),
      ..Run::new(0)
    });

    state.save_session();

    let session = state
      .database
      .load_session(state.session.id.unwrap())
      .unwrap();

    assert_eq!(
      session.transcript.messages(),
      [Message::User(vec![UserMessageContent::Text("foo".into())])]
    );
  }

  #[tokio::test]
  async fn stale_agent_events_do_not_mutate_new_run() {
    let mut state = State::new(&Settings {
      model: "mock:local".parse().unwrap(),
      prompt: Some("old".into()),
      yolo: false,
    })
    .unwrap();

    state.handle_event(Event::Action(Action::Submit));
    state.handle_event(Event::Action(Action::Interrupt));

    for c in "new".chars() {
      state.handle_event(Event::Action(Action::Edit(Input {
        key: Key::Char(c),
        ..Default::default()
      })));
    }

    assert_matches!(
      state.handle_event(Event::Action(Action::Submit)).as_slice(),
      [Effect::RunAgent { run_id: 1, .. }]
    );

    let invocation = ToolInvocation {
      id: "stale".into(),
      kind: ToolInvocationKind::Command(CommandTool {
        command: "echo".into(),
        cwd: None,
      }),
    };

    let (request, response_receiver) = ApprovalRequest::new(invocation.clone());

    for event in [
      AgentEvent::Delta("stale".into()),
      AgentEvent::ReasoningDelta("stale".into()),
      AgentEvent::Message(Message::Agent(vec![AgentMessageContent::ToolCall(
        invocation,
      )])),
      AgentEvent::Message(Message::User(vec![
        UserMessageContent::ToolResult {
          id: "stale".into(),
          result: ToolResult {
            content: Some("stale".into()),
            ..Default::default()
          },
        },
      ])),
      AgentEvent::ToolApprovalRequest(request),
      AgentEvent::Error("stale".into()),
      AgentEvent::Done,
    ] {
      state.handle_event(Event::Agent { event, run_id: 0 });
    }

    assert_eq!(
      response_receiver.await.unwrap_err().to_string(),
      "channel closed"
    );

    assert_eq!(state.approval(), None);

    assert_eq!(state.run, Some(Run::new(1)));

    state.handle_event(Event::Agent {
      event: AgentEvent::Delta("current".into()),
      run_id: 1,
    });

    state.handle_event(Event::Agent {
      event: AgentEvent::Done,
      run_id: 1,
    });

    assert_eq!(
      state.session.transcript.messages(),
      [
        Message::User(vec![UserMessageContent::Text("old".into())]),
        Message::User(vec![UserMessageContent::Text("new".into())]),
        Message::Agent(vec![AgentMessageContent::Text("current".into())]),
      ]
    );
  }

  #[test]
  fn submit_is_queued_while_agent_active() {
    let mut state = State::new(&Settings {
      model: "mock:local".parse().unwrap(),
      prompt: Some("foo".into()),
      yolo: false,
    })
    .unwrap();

    assert_eq!(
      state.handle_event(Event::Action(Action::Submit)),
      vec![Effect::RunAgent {
        messages: vec![Message::User(vec![UserMessageContent::Text(
          "foo".into()
        )])],
        run_id: 0,
      }]
    );

    for c in "bar".chars() {
      state.handle_event(Event::Action(Action::Edit(Input {
        key: Key::Char(c),
        ..Default::default()
      })));
    }

    assert_eq!(
      state.handle_event(Event::Action(Action::Submit)),
      Vec::new()
    );

    assert_eq!(state.composer.input_text(), "");
    assert_eq!(state.queued_inputs().len(), 1);

    assert_eq!(
      state.session.transcript.messages(),
      vec![Message::User(vec![UserMessageContent::Text("foo".into())])]
    );

    assert_eq!(
      state.handle_event(Event::Agent {
        event: AgentEvent::Done,
        run_id: 0,
      }),
      vec![Effect::RunAgent {
        messages: vec![
          Message::User(vec![UserMessageContent::Text("foo".into())]),
          Message::User(vec![UserMessageContent::Text("bar".into())]),
        ],
        run_id: 1,
      }]
    );

    assert!(state.queued_inputs().is_empty());
  }

  #[test]
  fn submit_trims_input() {
    #[track_caller]
    fn case(action: Action) {
      let mut state = State::new(&Settings {
        model: "mock:local".parse().unwrap(),
        prompt: Some("  foo  ".into()),
        yolo: false,
      })
      .unwrap();

      assert_eq!(
        state.handle_event(Event::Action(action)),
        vec![Effect::RunAgent {
          messages: vec![Message::User(vec![UserMessageContent::Text(
            "foo".into()
          )])],
          run_id: 0,
        }]
      );

      assert_eq!(state.composer.input_text(), "");

      state.handle_event(Event::Action(Action::SelectPrevious));

      assert_eq!(state.composer.input_text(), "foo");
    }

    case(Action::Submit);
    case(Action::SubmitImmediately);
  }

  #[test]
  fn terminal_error_interrupts_run() {
    let mut state = State::new(&Settings {
      model: "mock:local".parse().unwrap(),
      prompt: None,
      yolo: false,
    })
    .unwrap();

    state.run = Some(Run::new(0));

    assert_eq!(
      state.handle_event(Event::Error("foo".into())),
      vec![Effect::InterruptAgent]
    );
    assert_eq!(state.run, None);
    assert_eq!(
      state.session.transcript.entries,
      [TranscriptEntry::Error("foo".into())]
    );
  }

  #[test]
  fn terminal_error_without_run() {
    let mut state = State::new(&Settings {
      model: "mock:local".parse().unwrap(),
      prompt: None,
      yolo: false,
    })
    .unwrap();

    assert_eq!(state.handle_event(Event::Error("foo".into())), Vec::new());
    assert_eq!(state.run, None);
    assert_eq!(
      state.session.transcript.entries,
      [TranscriptEntry::Error("foo".into())]
    );
  }

  #[test]
  fn unknown_command() {
    let mut state = State::new(&Settings {
      model: "mock:local".parse().unwrap(),
      prompt: Some("/foobar".into()),
      yolo: false,
    })
    .unwrap();

    state.handle_event(Event::Action(Action::Submit));

    assert_matches!(
      &state.session.transcript.entries[..],
      [TranscriptEntry::Notice(notice)]
        if notice
          == "Unrecognized command '/foobar'. Type \"/\" for a list of supported commands."
    );

    assert_eq!(state.session.transcript.messages(), Vec::new());

    assert_eq!(state.composer.input_text(), "");
  }
}
