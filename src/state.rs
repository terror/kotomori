use super::*;

#[derive(Debug)]
pub(crate) struct State {
  pub(crate) composer: Composer,
  pub(crate) next_run_id: u64,
  pub(crate) queued_inputs: VecDeque<String>,
  pub(crate) reasoning_expanded: bool,
  pub(crate) run: Option<Run>,
  pub(crate) session: Session,
  pub(crate) should_quit: bool,
}

impl State {
  fn finish_run(&mut self, entry: Option<TranscriptEntry>) {
    let transcript = &mut self.session.transcript;

    transcript
      .entries
      .extend(self.run.take().and_then(Run::finish));

    transcript.interrupt_pending_calls();

    transcript.entries.extend(entry);
  }

  fn handle_action(&mut self, action: Action) -> Vec<Effect> {
    if action == Action::Quit && self.run.is_some() {
      return self.interrupt_agent();
    }

    match self.run.as_ref().and_then(|run| run.approval.as_ref()) {
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
        Action::ToggleReasoning => {
          self.reasoning_expanded = !self.reasoning_expanded;
        }
        Action::CompleteCommand
        | Action::Edit(_)
        | Action::Paste(_)
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
        Action::Paste(input) => self.composer.paste(&input),
        Action::Quit => self.quit(),
        Action::SelectNext => self.composer.select_next(),
        Action::SelectPrevious => self.composer.select_previous(),
        Action::Submit | Action::SubmitImmediately => {
          return self.submit(&action);
        }
        Action::ToggleReasoning => {
          self.reasoning_expanded = !self.reasoning_expanded;
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

        if interrupt_agent {
          vec![Effect::InterruptAgent, Effect::SaveSession]
        } else {
          vec![Effect::SaveSession]
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

            return once(Effect::SaveSession)
              .chain(self.run_next_queued())
              .collect();
          }
          AgentEvent::Update(update) => {
            run.update(update);
          }
          AgentEvent::Message(message) => {
            run.reset_message();
            self.session.transcript.push_message(message);
            return vec![Effect::SaveSession];
          }
          AgentEvent::Error(error) => {
            self.finish_run(Some(TranscriptEntry::Error(error)));

            return once(Effect::SaveSession)
              .chain(self.run_next_queued())
              .collect();
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

        return effects
          .into_iter()
          .chain(once(Effect::SaveSession))
          .collect();
      }
      Event::SessionSaved(result) => match result {
        Ok(saved) => {
          self.session.id = Some(saved.id);
          self.session.title = saved.title;
          self.session.updated_at = saved.updated_at;
        }
        Err(error) => self
          .session
          .transcript
          .error(format!("failed to save session: {error}")),
      },
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

    vec![Effect::InterruptAgent, Effect::SaveSession]
  }

  pub(crate) fn new(session: Session) -> Self {
    let history = session
      .transcript
      .entries
      .iter()
      .filter_map(TranscriptEntry::message)
      .filter_map(Message::user_content)
      .map(str::to_owned)
      .collect();

    Self {
      composer: Composer::new(
        session.settings.prompt.as_deref().unwrap_or_default(),
        history,
      ),
      next_run_id: 0,
      queued_inputs: VecDeque::new(),
      reasoning_expanded: false,
      run: None,
      session,
      should_quit: false,
    }
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

  fn submit(&mut self, action: &Action) -> Vec<Effect> {
    let input = self.composer.input_text();
    let trimmed = input.trim();

    if let Some(command) =
      Command::from_input(trimmed).or_else(|| self.composer.selected_command())
    {
      return self.handle_command(command);
    }

    if trimmed.starts_with('/') {
      if trimmed.len() > 1 {
        self.session.transcript.notice(format!(
          "Unrecognized command '{trimmed}'. Type \"/\" for a list of supported commands."
        ));

        self.reset_input();
      }

      return Vec::new();
    }

    if trimmed.is_empty() {
      return Vec::new();
    }

    self.composer.remember(&input);
    self.reset_input();

    match action {
      Action::SubmitImmediately if self.run.is_some() => self
        .interrupt_agent()
        .into_iter()
        .chain(once(self.run(input)))
        .collect(),
      _ if self.run.is_some() => {
        self.queued_inputs.push_back(input);
        Vec::new()
      }
      _ => vec![Effect::SaveSession, self.run(input)],
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn agent_events_update_transcript() {
    let mut state = State::new(Session::new(
      &Settings {
        directory: "foo".into(),
        model: "mock:local".parse().unwrap(),
        prompt: Some("foo".into()),
        yolo: false,
      },
      0,
    ));

    assert_eq!(
      state.handle_event(Event::Action(Action::Submit)),
      vec![
        Effect::SaveSession,
        Effect::RunAgent {
          messages: vec![Message::User(vec![UserMessageContent::Text(
            "foo".into()
          )])],
          run_id: 0,
        }
      ]
    );

    state.handle_event(Event::Agent {
      event: AgentEvent::Update(MessageUpdate::ReasoningDelta {
        delta: "bar".into(),
        index: 0,
      }),
      run_id: 0,
    });
    state.handle_event(Event::Agent {
      event: AgentEvent::Update(MessageUpdate::Text {
        delta: "baz".into(),
        index: 1,
      }),
      run_id: 0,
    });

    let invocation = ToolInvocation::new(
      "foo",
      ToolInvocationKind::Command(CommandTool {
        command: "bar".into(),
        cwd: None,
      }),
    );

    assert_eq!(
      state.handle_event(Event::Agent {
        event: AgentEvent::Message(Message::agent(vec![
          AssistantContent::reasoning("foo", "bar"),
          AssistantContent::text("baz"),
          AssistantContent::ToolCall(invocation.protocol.clone()),
        ])),
        run_id: 0,
      }),
      vec![Effect::SaveSession]
    );

    let result = ToolResult {
      exit_status: Some(0),
      outcome: ToolOutcome::Success,
      stdout: Some("qux".into()),
      ..Default::default()
    };

    state.handle_event(Event::Agent {
      event: AgentEvent::Message(Message::User(vec![
        UserMessageContent::ToolResult {
          call: CallId::from_wire("foo"),
          name: ToolName::new("command").unwrap(),
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
        Message::agent(vec![
          AssistantContent::reasoning("foo", "bar"),
          AssistantContent::text("baz"),
          AssistantContent::ToolCall(invocation.protocol),
        ]),
        Message::User(vec![UserMessageContent::ToolResult {
          call: CallId::from_wire("foo"),
          name: ToolName::new("command").unwrap(),
          result,
        }]),
      ],
    );
  }

  #[test]
  fn agent_events_without_run_are_ignored() {
    let mut state = State::new(Session::new(
      &Settings {
        directory: "foo".into(),
        model: "mock:local".parse().unwrap(),
        prompt: None,
        yolo: false,
      },
      0,
    ));

    let invocation = ToolInvocation::new(
      "foo",
      ToolInvocationKind::Command(CommandTool {
        command: "bar".into(),
        cwd: None,
      }),
    );

    for event in [
      AgentEvent::Message(Message::agent(vec![AssistantContent::ToolCall(
        invocation.protocol,
      )])),
      AgentEvent::Message(Message::User(vec![
        UserMessageContent::ToolResult {
          call: CallId::from_wire("foo"),
          name: ToolName::new("command").unwrap(),
          result: ToolResult::default(),
        },
      ])),
      AgentEvent::Done,
    ] {
      assert_eq!(
        state.handle_event(Event::Agent { event, run_id: 0 }),
        Vec::new(),
      );
    }

    assert_eq!(state.run, None);
    assert_eq!(state.session.transcript.entries, []);
  }

  #[tokio::test]
  async fn approval_terminal_agent_tool_result_drops_pending_request() {
    let mut state = State::new(Session::new(
      &Settings {
        directory: "foo".into(),
        model: "mock:local".parse().unwrap(),
        prompt: Some(String::new()),
        yolo: false,
      },
      0,
    ));

    let (request, response_receiver) =
      ApprovalRequest::new(ToolInvocation::new(
        "foo",
        ToolInvocationKind::Command(CommandTool {
          command: "bar".into(),
          cwd: None,
        }),
      ));

    state.run = Some(Run {
      approval: Some(request),
      ..Run::new(0)
    });

    state.handle_event(Event::Agent {
      event: AgentEvent::Message(Message::User(vec![
        UserMessageContent::ToolResult {
          call: CallId::from_wire("foo"),
          name: ToolName::new("command").unwrap(),
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
    let mut state = State::new(Session::new(
      &Settings {
        directory: "foo".into(),
        model: "mock:local".parse().unwrap(),
        prompt: None,
        yolo: false,
      },
      0,
    ));

    let (request, response_receiver) =
      ApprovalRequest::new(ToolInvocation::new(
        "foo",
        ToolInvocationKind::Command(CommandTool {
          command: "bar".into(),
          cwd: None,
        }),
      ));

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
  fn completed_messages_survive_resumption() {
    let settings = Settings {
      directory: "foo".into(),
      model: "mock:local".parse().unwrap(),
      prompt: Some("foo".into()),
      yolo: false,
    };

    let mut state = State::new(Session::new(&settings, 0));

    state.handle_event(Event::Action(Action::Submit));

    state.handle_event(Event::Agent {
      event: AgentEvent::Update(MessageUpdate::Text {
        delta: "bar".into(),
        index: 0,
      }),
      run_id: 0,
    });

    state.handle_event(Event::Agent {
      event: AgentEvent::Update(MessageUpdate::ReasoningDelta {
        delta: "baz".into(),
        index: 1,
      }),
      run_id: 0,
    });

    let messages = vec![
      Message::agent(vec![
        AssistantContent::reasoning("foo", "foo"),
        AssistantContent::text("bar"),
        AssistantContent::ToolCall(
          ToolInvocation::new(
            "foo",
            ToolInvocationKind::Command(CommandTool {
              command: "bar".into(),
              cwd: None,
            }),
          )
          .protocol,
        ),
        AssistantContent::text("baz"),
        AssistantContent::ToolCall(
          ToolInvocation::new(
            "bar",
            ToolInvocationKind::Command(CommandTool {
              command: "qux".into(),
              cwd: None,
            }),
          )
          .protocol,
        ),
      ]),
      Message::User(vec![
        UserMessageContent::ToolResult {
          call: CallId::from_wire("bar"),
          name: ToolName::new("command").unwrap(),
          result: ToolResult {
            content: Some("foo".into()),
            ..Default::default()
          },
        },
        UserMessageContent::ToolResult {
          call: CallId::from_wire("foo"),
          name: ToolName::new("command").unwrap(),
          result: ToolResult {
            content: Some("bar".into()),
            ..Default::default()
          },
        },
      ]),
      Message::agent(vec![AssistantContent::text("qux")]),
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

    let session = state.session;

    let mut state = State::new(session);

    let messages =
      once(Message::User(vec![UserMessageContent::Text("foo".into())]))
        .chain(messages)
        .chain(once(Message::User(vec![UserMessageContent::Text(
          "foo".into(),
        )])))
        .collect();

    assert_eq!(
      state.handle_event(Event::Action(Action::Submit)),
      [
        Effect::SaveSession,
        Effect::RunAgent {
          messages,
          run_id: 0
        }
      ]
    );
  }

  #[test]
  fn error_is_not_included_in_next_request() {
    let mut state = State::new(Session::new(
      &Settings {
        directory: "foo".into(),
        model: "mock:local".parse().unwrap(),
        prompt: Some("foo".into()),
        yolo: false,
      },
      0,
    ));

    assert_eq!(
      state.handle_event(Event::Action(Action::Submit)),
      vec![
        Effect::SaveSession,
        Effect::RunAgent {
          messages: vec![Message::User(vec![UserMessageContent::Text(
            "foo".into()
          )])],
          run_id: 0,
        }
      ]
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
      vec![
        Effect::SaveSession,
        Effect::RunAgent {
          messages: vec![
            Message::User(vec![UserMessageContent::Text("foo".into())]),
            Message::User(vec![UserMessageContent::Text("baz".into())]),
          ],
          run_id: 1,
        }
      ]
    );
  }

  #[test]
  fn failed_save_preserves_run() {
    let mut state = State::new(Session::new(
      &Settings {
        directory: "foo".into(),
        model: "mock:local".parse().unwrap(),
        prompt: None,
        yolo: false,
      },
      0,
    ));

    state.run = Some({
      let mut run = Run::new(0);
      run.update(MessageUpdate::Text {
        delta: "foo".into(),
        index: 0,
      });
      run
    });

    state.session.id = Some(0);

    assert_eq!(
      state.handle_event(Event::SessionSaved(Err("foo".into()))),
      Vec::new()
    );

    assert_eq!(
      state.run,
      Some({
        let mut run = Run::new(0);
        run.update(MessageUpdate::Text {
          delta: "foo".into(),
          index: 0,
        });
        run
      })
    );
    assert_eq!(
      state.session.transcript.entries,
      [TranscriptEntry::Error("failed to save session: foo".into())]
    );
  }

  #[test]
  fn finish_run_saves_partial_output() {
    let mut state = State::new(Session::new(
      &Settings {
        directory: "foo".into(),
        model: "mock:local".parse().unwrap(),
        prompt: None,
        yolo: false,
      },
      0,
    ));

    state.run = Some({
      let mut run = Run::new(0);
      run.update(MessageUpdate::Text {
        delta: "foo".into(),
        index: 0,
      });
      run
    });

    assert_eq!(
      state.handle_event(Event::Agent {
        event: AgentEvent::Done,
        run_id: 0
      }),
      [Effect::SaveSession]
    );

    assert_eq!(state.run, None);
    assert_eq!(
      state.session.transcript.messages(),
      [Message::agent(vec![AssistantContent::text("foo")])]
    );
  }

  #[test]
  fn interruption_preserves_streamed_protocol() {
    let mut state = State::new(Session::new(
      &Settings {
        directory: "foo".into(),
        model: "mock:foo".parse().unwrap(),
        prompt: None,
        yolo: false,
      },
      0,
    ));

    state.run = Some(Run::new(0));

    let (event_sender, mut events) = mpsc::unbounded_channel();

    let sink = ProviderSink {
      event_sender,
      run_id: 0,
    };

    let reasoning = Reasoning::new_with_signature("foo", Some("bar".into()))
      .with_id("baz".into())
      .sealed("foo");

    let encrypted = Reasoning::encrypted("qux")
      .with_id("quux".into())
      .sealed("foo");

    for update in [
      MessageUpdate::ReasoningDelta {
        delta: "foo".into(),
        index: 0,
      },
      MessageUpdate::Content {
        index: 0,
        content: AssistantContent::Reasoning(reasoning.clone()),
      },
      MessageUpdate::Content {
        index: 1,
        content: AssistantContent::Reasoning(encrypted.clone()),
      },
      MessageUpdate::Text {
        delta: "quuz".into(),
        index: 2,
      },
    ] {
      sink.update(update).unwrap();
    }

    while let Ok(event) = events.try_recv() {
      state.handle_event(event);
    }

    let content = vec![
      AssistantContent::Reasoning(reasoning),
      AssistantContent::Reasoning(encrypted),
      AssistantContent::text("quuz"),
    ];

    state.handle_event(Event::Action(Action::Interrupt));

    let session = state.session;

    assert_eq!(
      session.transcript.entries,
      [
        TranscriptEntry::Message(Message::agent(content)),
        TranscriptEntry::Interrupted,
      ]
    );
  }

  #[test]
  fn new_uses_empty_prompt_by_default() {
    let state = State::new(Session::new(
      &Settings {
        directory: "foo".into(),
        model: "mock:local".parse().unwrap(),
        prompt: None,
        yolo: false,
      },
      0,
    ));

    assert_eq!(state.composer.input_text(), "");
  }

  #[tokio::test]
  async fn quit_interrupts_active_approval() {
    let mut state = State::new(Session::new(
      &Settings {
        directory: "foo".into(),
        model: "mock:local".parse().unwrap(),
        prompt: Some("foo".into()),
        yolo: false,
      },
      0,
    ));

    state.handle_event(Event::Action(Action::Submit));

    let (request, response_receiver) =
      ApprovalRequest::new(ToolInvocation::new(
        "foo",
        ToolInvocationKind::Command(CommandTool {
          command: "bar".into(),
          cwd: None,
        }),
      ));

    state.handle_event(Event::Agent {
      event: AgentEvent::ToolApprovalRequest(request),
      run_id: 0,
    });

    assert!(state.run.as_ref().unwrap().approval.is_some());

    assert_eq!(
      state.handle_event(Event::Action(Action::Quit)),
      vec![Effect::InterruptAgent, Effect::SaveSession]
    );

    assert!(!state.should_quit);
    assert_eq!(state.run, None);

    assert_eq!(
      state.run.as_ref().and_then(|run| run.approval.as_ref()),
      None
    );
    assert!(response_receiver.await.is_err());
  }

  #[test]
  fn session_excludes_streaming_content() {
    let mut state = State::new(Session::new(
      &Settings {
        directory: "foo".into(),
        model: "mock:local".parse().unwrap(),
        prompt: None,
        yolo: false,
      },
      0,
    ));

    state.session.transcript.send("foo".into());

    state.run = Some({
      let mut run = Run::new(0);
      run.update(MessageUpdate::Text {
        delta: "bar".into(),
        index: 0,
      });
      run
    });

    let session = state.session;

    assert_eq!(
      session.transcript.messages(),
      [Message::User(vec![UserMessageContent::Text("foo".into())])]
    );
  }

  #[tokio::test]
  async fn stale_agent_events_do_not_mutate_new_run() {
    let mut state = State::new(Session::new(
      &Settings {
        directory: "foo".into(),
        model: "mock:local".parse().unwrap(),
        prompt: Some("old".into()),
        yolo: false,
      },
      0,
    ));

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
      [Effect::SaveSession, Effect::RunAgent { run_id: 1, .. }]
    );

    let invocation = ToolInvocation::new(
      "stale",
      ToolInvocationKind::Command(CommandTool {
        command: "echo".into(),
        cwd: None,
      }),
    );

    let (request, response_receiver) = ApprovalRequest::new(invocation.clone());

    for event in [
      AgentEvent::Update(MessageUpdate::Text {
        delta: "stale".into(),
        index: 0,
      }),
      AgentEvent::Update(MessageUpdate::ReasoningDelta {
        delta: "stale".into(),
        index: 0,
      }),
      AgentEvent::Message(Message::agent(vec![AssistantContent::ToolCall(
        invocation.protocol,
      )])),
      AgentEvent::Message(Message::User(vec![
        UserMessageContent::ToolResult {
          call: CallId::from_wire("stale"),
          name: ToolName::new("command").unwrap(),
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

    assert_eq!(
      state.run.as_ref().and_then(|run| run.approval.as_ref()),
      None
    );

    assert_eq!(state.run, Some(Run::new(1)));

    state.handle_event(Event::Agent {
      event: AgentEvent::Update(MessageUpdate::Text {
        delta: "current".into(),
        index: 0,
      }),
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
        Message::agent(vec![AssistantContent::text("current")]),
      ]
    );
  }

  #[test]
  fn successful_save_updates_metadata_without_saving_again() {
    let settings = Settings {
      directory: "foo".into(),
      model: "mock:foo".parse().unwrap(),
      prompt: None,
      yolo: false,
    };

    let mut state = State::new(Session::new(&settings, 1));

    state.session.transcript.send("bar".into());

    assert_eq!(
      state.handle_event(Event::SessionSaved(Ok(SavedSession {
        id: 2,
        title: Some("bar".into()),
        updated_at: 3,
      }))),
      Vec::new(),
    );
    assert_eq!(
      state.session,
      Session {
        created_at: 1,
        id: Some(2),
        settings,
        title: Some("bar".into()),
        transcript: Transcript::with_entries(vec![TranscriptEntry::Message(
          Message::User(vec![UserMessageContent::Text("bar".into())]),
        )]),
        updated_at: 3,
      }
    );
    assert_eq!(state.session.settings.directory, Path::new("foo"));
  }

  #[test]
  fn terminal_error_interrupts_run() {
    let mut state = State::new(Session::new(
      &Settings {
        directory: "foo".into(),
        model: "mock:local".parse().unwrap(),
        prompt: None,
        yolo: false,
      },
      0,
    ));

    state.run = Some(Run::new(0));

    assert_eq!(
      state.handle_event(Event::Error("foo".into())),
      vec![Effect::InterruptAgent, Effect::SaveSession]
    );
    assert_eq!(state.run, None);
    assert_eq!(
      state.session.transcript.entries,
      [TranscriptEntry::Error("foo".into())]
    );
  }

  #[test]
  fn terminal_error_without_run() {
    let mut state = State::new(Session::new(
      &Settings {
        directory: "foo".into(),
        model: "mock:local".parse().unwrap(),
        prompt: None,
        yolo: false,
      },
      0,
    ));

    assert_eq!(
      state.handle_event(Event::Error("foo".into())),
      vec![Effect::SaveSession]
    );
    assert_eq!(state.run, None);
    assert_eq!(
      state.session.transcript.entries,
      [TranscriptEntry::Error("foo".into())]
    );
  }

  #[test]
  fn unfinished_reasoning_survives_without_entering_requests() {
    #[track_caller]
    fn case(event: Event, entry: TranscriptEntry) {
      let settings = Settings {
        directory: "foo".into(),
        model: "mock:foo".parse().unwrap(),
        prompt: Some("qux".into()),
        yolo: false,
      };

      let mut state = State::new(Session::new(&settings, 0));

      let reasoning =
        Reasoning::new_with_signature("foo", Some("bar".into())).sealed("foo");

      let mut run = Run::new(0);

      run.update_many(&[
        MessageUpdate::Content {
          index: 0,
          content: AssistantContent::Reasoning(reasoning.clone()),
        },
        MessageUpdate::Text {
          delta: "baz".into(),
          index: 1,
        },
        MessageUpdate::ReasoningDelta {
          delta: "quux".into(),
          index: 2,
        },
      ]);

      let draft = run.message.clone();

      state.run = Some(run);
      state.handle_event(event);

      let session = state.session;

      assert_eq!(
        session.transcript.entries,
        [TranscriptEntry::Draft(draft), entry]
      );

      let mut state = State::new(session);

      assert_eq!(
        state.handle_event(Event::Action(Action::Submit)),
        [
          Effect::SaveSession,
          Effect::RunAgent {
            messages: vec![
              Message::agent(vec![
                AssistantContent::Reasoning(reasoning),
                AssistantContent::text("baz"),
              ]),
              Message::User(vec![UserMessageContent::Text("qux".into())]),
            ],
            run_id: 0,
          }
        ]
      );
    }

    case(
      Event::Action(Action::Interrupt),
      TranscriptEntry::Interrupted,
    );

    case(
      Event::Agent {
        event: AgentEvent::Error("foo".into()),
        run_id: 0,
      },
      TranscriptEntry::Error("foo".into()),
    );
  }
}
