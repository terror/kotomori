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
  fn clear(&mut self) {
    self.run = None;
    self.session.clear();
    self.composer.clear_history();
    self.queued_inputs.clear();
  }

  fn finish_run(&mut self, run_id: u64, result: RunResult) -> Vec<Effect> {
    if self.run.as_ref().is_none_or(|run| run.id != run_id) {
      return Vec::new();
    }

    let run = self.run.take().unwrap();

    let run_next =
      matches!(run.interruption, None | Some(Interruption::ContinueQueued));

    if run.interruption == Some(Interruption::Clear) {
      self.clear();
    } else {
      match result.outcome {
        RunOutcome::Cancelled => {
          if !matches!(run.interruption, Some(Interruption::Error(_))) {
            self.session.push(TranscriptEntry::Interrupted);
          }
        }
        RunOutcome::Completed => {}
        RunOutcome::Failed(error) => {
          self.session.push(TranscriptEntry::Error(error));
        }
        RunOutcome::Limited(limit) => {
          self.session.push(TranscriptEntry::Error(limit.to_string()));
        }
      }

      if let Some(Interruption::Error(error)) = run.interruption {
        self.session.push(TranscriptEntry::Error(error));
      }
    }

    once(Effect::SaveSession)
      .chain(if run_next {
        self.run_next_queued()
      } else {
        Vec::new()
      })
      .collect()
  }

  fn handle_action(&mut self, action: Action) -> Vec<Effect> {
    if action == Action::Quit && self.run.is_some() {
      return self.interrupt_agent(Interruption::Stop);
    }

    match self.run.as_ref().and_then(|run| run.approval.as_ref()) {
      Some(_) => match action {
        Action::Edit(input) if input.key == Key::Char('y') => {
          return self.resolve_approval(ToolApproval::Approved);
        }
        Action::Edit(input) if input.key == Key::Char('Y') => {
          return self.resolve_approval(ToolApproval::Approved);
        }
        Action::Edit(input) if input.key == Key::Char('n') => {
          return self.resolve_approval(ToolApproval::Denied);
        }
        Action::Edit(input) if input.key == Key::Char('N') => {
          return self.resolve_approval(ToolApproval::Denied);
        }
        Action::Interrupt => {
          return self.resolve_approval(ToolApproval::Denied);
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
          return self.interrupt_agent(Interruption::ContinueQueued);
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

  fn handle_command(
    &mut self,
    command: Command,
    arguments: &str,
  ) -> Vec<Effect> {
    let effects = match command {
      Command::Clear => {
        if self.run.is_some() {
          self.interrupt_agent(Interruption::Clear)
        } else {
          self.clear();

          vec![Effect::SaveSession]
        }
      }
      Command::Copy => {
        let response = self
          .run
          .as_ref()
          .and_then(|run| run.message.message().text())
          .or_else(|| self.session.conversation().last_response());

        if let Some(response) = response {
          vec![Effect::CopyToClipboard(response)]
        } else {
          self.session.push(TranscriptEntry::Notice(
            "No assistant response to copy.".into(),
          ));

          Vec::new()
        }
      }
      Command::Fork => self
        .interrupt_agent(Interruption::Stop)
        .into_iter()
        .chain(once(Effect::ForkSession))
        .collect(),
      Command::Quit => self.handle_action(Action::Quit),
      Command::Rename => {
        if arguments.is_empty() {
          self
            .session
            .push(TranscriptEntry::Notice("Usage: /rename <title>".into()));

          Vec::new()
        } else {
          let title =
            arguments.split_whitespace().collect::<Vec<_>>().join(" ");

          self.session.push(TranscriptEntry::Notice(format!(
            "Session renamed to '{title}'."
          )));

          self.session.title = Some(title);

          vec![Effect::SaveSession]
        }
      }
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
          AgentEvent::Update { update, .. } => {
            run.update(update);
          }
          AgentEvent::Message { .. } => {
            run.reset_message();

            return vec![Effect::SaveSession];
          }
          AgentEvent::ToolApprovalRequest(request) => {
            run.approval = Some(request);
          }
          AgentEvent::ResponseStarted(_) => {
            run.activity = AgentActivity::Waiting;
          }
          AgentEvent::ToolFinished { call, .. } => {
            if run.running_tool.as_ref() == Some(&call) {
              run.running_tool = None;
            }
          }
          AgentEvent::ToolStarted { call, .. } => {
            run.running_tool = Some(call);
          }
          AgentEvent::ResponseFinished(_) | AgentEvent::ToolProgress { .. } => {
          }
        }
      }
      Event::AgentCompleted { result, run_id } => {
        return self.finish_run(run_id, result);
      }
      Event::ClipboardCopied(result) => match result {
        Ok(()) => self.session.push(TranscriptEntry::Notice(
          "Copied last assistant response to clipboard.".into(),
        )),
        Err(error) => self.session.push(TranscriptEntry::Error(format!(
          "failed to copy response to clipboard: {error}"
        ))),
      },
      Event::Error(error) => {
        if self.run.is_some() {
          return self.interrupt_agent(Interruption::Error(error));
        }

        self.session.push(TranscriptEntry::Error(error));

        return vec![Effect::SaveSession];
      }
      Event::SessionForked(result) => match result {
        Ok(session) => {
          self.session = session;

          self
            .session
            .push(TranscriptEntry::Notice("Forked conversation.".into()));

          self.queued_inputs.clear();
        }
        Err(error) => self.session.push(TranscriptEntry::Error(format!(
          "failed to fork session: {error}"
        ))),
      },
      Event::SessionSaved(result) => match result {
        Ok(saved) => {
          self.session.id = Some(saved.id);
          self.session.title = saved.title;
          self.session.updated_at = saved.updated_at;
        }
        Err(error) => self.session.push(TranscriptEntry::Error(format!(
          "failed to save session: {error}"
        ))),
      },
      Event::Tick(elapsed) => {
        if let Some(run) = &mut self.run {
          run.tick(elapsed);
        }
      }
    }

    Vec::new()
  }

  fn interrupt_agent(&mut self, interruption: Interruption) -> Vec<Effect> {
    let Some(run) = &mut self.run else {
      return Vec::new();
    };

    run.interruption = Some(interruption);

    vec![Effect::InterruptAgent]
  }

  pub(crate) fn new(mut session: Session) -> Result<Self> {
    session.interrupt_pending_calls()?;

    let history = session
      .conversation()
      .messages()
      .iter()
      .filter_map(Message::user_content)
      .map(str::to_owned)
      .collect();

    Ok(Self {
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
    })
  }

  fn quit(&mut self) {
    self.should_quit = true;
  }

  fn reset_input(&mut self) {
    self.composer.clear();
  }

  fn resolve_approval(&mut self, approval: ToolApproval) -> Vec<Effect> {
    self
      .run
      .as_mut()
      .and_then(|run| run.approval.take())
      .map(|request| Effect::ResolveApproval {
        approval,
        id: request.id,
      })
      .into_iter()
      .collect()
  }

  fn run(&mut self, input: String) -> Option<Effect> {
    if let Err(error) = self.session.core.prompt(input) {
      self.session.push(TranscriptEntry::Error(error.to_string()));
      return None;
    }

    self.session.refresh();

    let run_id = self.next_run_id;

    self.next_run_id = self
      .next_run_id
      .checked_add(1)
      .expect("agent run ID overflow");

    self.run = Some(Run::new(run_id));

    Some(Effect::RunAgent { run_id })
  }

  fn run_next_queued(&mut self) -> Vec<Effect> {
    self
      .queued_inputs
      .pop_front()
      .and_then(|input| self.run(input))
      .into_iter()
      .collect()
  }

  fn submit(&mut self, action: &Action) -> Vec<Effect> {
    let input = self.composer.input_text();
    let trimmed = input.trim();

    if let Some((command, arguments)) =
      Command::from_input(trimmed).or_else(|| {
        self
          .composer
          .selected_command()
          .map(|command| (command, ""))
      })
    {
      return self.handle_command(command, arguments);
    }

    if trimmed.starts_with('/') {
      if trimmed.len() > 1 {
        self.session.push(TranscriptEntry::Notice(format!(
          "Unrecognized command '{trimmed}'. Type \"/\" for a list of supported commands."
        )));

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
      Action::SubmitImmediately if self.run.is_some() => {
        self.queued_inputs.push_front(input);
        self.interrupt_agent(Interruption::ContinueQueued)
      }
      _ if self.run.is_some() => {
        self.queued_inputs.push_back(input);
        Vec::new()
      }
      _ => once(Effect::SaveSession).chain(self.run(input)).collect(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  struct TestEvent;

  impl TestEvent {
    fn approval(invocation: ToolRequest) -> ApprovalRequest {
      ApprovalRequest {
        id: ApprovalId {
          call: invocation.id.clone(),
          response: Self::response(),
        },
        invocation,
      }
    }

    fn message(message: Message) -> AgentEvent {
      let response = Self::response();

      AgentEvent::Message {
        message,
        response: Some(response),
        run: response.run,
      }
    }

    fn response() -> ResponseId {
      serde_json::from_value::<ResponseId>(serde_json::json!({
        "round": 1,
        "run": 1,
      }))
      .unwrap()
    }

    fn update(update: MessageUpdate) -> AgentEvent {
      AgentEvent::Update {
        response: Self::response(),
        update,
      }
    }
  }

  #[test]
  fn agent_events_without_run_are_ignored() {
    let mut state = State::new(Session::new(
      &Settings {
        directory: "foo".into(),
        max_output_tokens: None,
        max_rounds: AgentConfig::DEFAULT_MAX_ROUNDS,
        model: "mock:local".parse().unwrap(),
        prompt: None,
        yolo: false,
      },
      0,
    ))
    .unwrap();

    let invocation = ToolRequest::new(
      CallId::new("foo"),
      ToolFunction {
        arguments: serde_json::json!({"command": "bar"}),
        name: ToolName::new("command").unwrap(),
      },
    );

    for event in [
      TestEvent::message(Message::Agent(
        vec![AssistantContent::ToolCall(invocation)].into(),
      )),
      TestEvent::message(Message::User(vec![UserMessageContent::ToolResult {
        call: CallId::new("foo"),
        name: ToolName::new("command").unwrap(),
        result: ToolResult::default(),
      }])),
    ] {
      assert_eq!(
        state.handle_event(Event::Agent { event, run_id: 0 }),
        Vec::new(),
      );
    }

    assert_eq!(
      state.handle_event(Event::AgentCompleted {
        result: RunState::new(state.session.conversation().clone())
          .finish(RunOutcome::Completed),
        run_id: 0,
      }),
      Vec::new(),
    );

    assert_eq!(state.run, None);
    assert_eq!(state.session.transcript, []);
  }

  #[test]
  fn approval_terminal_agent_tool_result_drops_pending_request() {
    let mut state = State::new(Session::new(
      &Settings {
        directory: "foo".into(),
        max_output_tokens: None,
        max_rounds: AgentConfig::DEFAULT_MAX_ROUNDS,
        model: "mock:local".parse().unwrap(),
        prompt: Some(String::new()),
        yolo: false,
      },
      0,
    ))
    .unwrap();

    let request = TestEvent::approval(ToolRequest::new(
      CallId::new("foo"),
      ToolFunction {
        arguments: serde_json::json!({"command": "bar"}),
        name: ToolName::new("command").unwrap(),
      },
    ));

    state.run = Some(Run {
      approval: Some(request),
      ..Run::new(0)
    });

    state.handle_event(Event::Agent {
      event: TestEvent::message(Message::User(vec![
        UserMessageContent::ToolResult {
          call: CallId::new("foo"),
          name: ToolName::new("command").unwrap(),
          result: ToolResult {
            content: "bar".into(),
            ..Default::default()
          },
        },
      ])),
      run_id: 0,
    });

    assert_eq!(state.run, Some(Run::new(0)));
  }

  #[test]
  fn approval_terminal_error_drops_pending_request() {
    let mut state = State::new(Session::new(
      &Settings {
        directory: "foo".into(),
        max_output_tokens: None,
        max_rounds: AgentConfig::DEFAULT_MAX_ROUNDS,
        model: "mock:local".parse().unwrap(),
        prompt: None,
        yolo: false,
      },
      0,
    ))
    .unwrap();

    let request = TestEvent::approval(ToolRequest::new(
      CallId::new("foo"),
      ToolFunction {
        arguments: serde_json::json!({"command": "bar"}),
        name: ToolName::new("command").unwrap(),
      },
    ));

    state.run = Some(Run {
      approval: Some(request),
      ..Run::new(0)
    });

    state.handle_event(Event::AgentCompleted {
      result: RunState::new(state.session.conversation().clone())
        .finish(RunOutcome::Failed("bar".into())),
      run_id: 0,
    });

    assert_eq!(state.run, None);
  }

  #[test]

  fn error_is_not_included_in_next_request() {
    let mut state = State::new(Session::new(
      &Settings {
        directory: "foo".into(),
        max_output_tokens: None,
        max_rounds: AgentConfig::DEFAULT_MAX_ROUNDS,
        model: "mock:local".parse().unwrap(),
        prompt: Some("foo".into()),
        yolo: false,
      },
      0,
    ))
    .unwrap();

    assert_eq!(
      state.handle_event(Event::Action(Action::Submit)),
      vec![Effect::SaveSession, Effect::RunAgent { run_id: 0 }]
    );

    state.handle_event(Event::AgentCompleted {
      result: RunState::new(state.session.conversation().clone())
        .finish(RunOutcome::Failed("bar".into())),
      run_id: 0,
    });

    assert_eq!(
      state.session.conversation().messages(),
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
      vec![Effect::SaveSession, Effect::RunAgent { run_id: 1 }]
    );
  }

  #[test]
  fn failed_save_preserves_run() {
    let mut state = State::new(Session::new(
      &Settings {
        directory: "foo".into(),
        max_output_tokens: None,
        max_rounds: AgentConfig::DEFAULT_MAX_ROUNDS,
        model: "mock:local".parse().unwrap(),
        prompt: None,
        yolo: false,
      },
      0,
    ))
    .unwrap();

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
      state.session.transcript,
      [TranscriptEntry::Error("failed to save session: foo".into())]
    );
  }

  #[test]
  fn stale_agent_events_do_not_mutate_new_run() {
    let mut state = State::new(Session::new(
      &Settings {
        directory: "foo".into(),
        max_output_tokens: None,
        max_rounds: AgentConfig::DEFAULT_MAX_ROUNDS,
        model: "mock:local".parse().unwrap(),
        prompt: Some("old".into()),
        yolo: false,
      },
      0,
    ))
    .unwrap();

    state.handle_event(Event::Action(Action::Submit));
    state.handle_event(Event::Action(Action::Interrupt));

    state.handle_event(Event::AgentCompleted {
      result: RunState::new(state.session.conversation().clone())
        .finish(RunOutcome::Cancelled),
      run_id: 0,
    });

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

    let invocation = ToolRequest::new(
      CallId::new("stale"),
      ToolFunction {
        arguments: serde_json::json!({"command": "echo"}),
        name: ToolName::new("command").unwrap(),
      },
    );

    let request = TestEvent::approval(invocation.clone());

    for event in [
      TestEvent::update(MessageUpdate::Text {
        delta: "stale".into(),
        index: 0,
      }),
      TestEvent::update(MessageUpdate::ReasoningDelta {
        delta: "stale".into(),
        index: 0,
      }),
      TestEvent::message(Message::Agent(
        vec![AssistantContent::ToolCall(invocation)].into(),
      )),
      TestEvent::message(Message::User(vec![UserMessageContent::ToolResult {
        call: CallId::new("stale"),
        name: ToolName::new("command").unwrap(),
        result: ToolResult {
          content: "stale".into(),
          ..Default::default()
        },
      }])),
      AgentEvent::ToolApprovalRequest(request),
    ] {
      state.handle_event(Event::Agent { event, run_id: 0 });
    }

    for outcome in [RunOutcome::Failed("stale".into()), RunOutcome::Completed] {
      state.handle_event(Event::AgentCompleted {
        result: RunState::default().finish(outcome),
        run_id: 0,
      });
    }

    assert_eq!(
      state.run.as_ref().and_then(|run| run.approval.as_ref()),
      None
    );

    assert_eq!(state.run, Some(Run::new(1)));

    state.handle_event(Event::Agent {
      event: TestEvent::update(MessageUpdate::Text {
        delta: "current".into(),
        index: 0,
      }),
      run_id: 1,
    });

    let result = RunState {
      conversation: state.session.conversation().clone(),
      message: state.run.as_ref().unwrap().message.clone(),
      ..Default::default()
    }
    .finish(RunOutcome::Completed);

    state.handle_event(Event::AgentCompleted { result, run_id: 1 });

    assert_eq!(
      state.session.conversation().messages(),
      [
        Message::User(vec![UserMessageContent::Text("old".into())]),
        Message::User(vec![UserMessageContent::Text("new".into())]),
      ]
    );
  }

  #[test]
  fn successful_save_updates_metadata_without_saving_again() {
    let settings = Settings {
      directory: "foo".into(),
      max_output_tokens: None,
      max_rounds: AgentConfig::DEFAULT_MAX_ROUNDS,
      model: "mock:foo".parse().unwrap(),
      prompt: None,
      yolo: false,
    };

    let mut state = State::new(Session::new(&settings, 1)).unwrap();

    state
      .session
      .push(TranscriptEntry::Message(Message::User(vec![
        UserMessageContent::Text("bar".into()),
      ])));

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
        core: SessionSnapshot::new(vec![
          RunEntry::Message(Message::User(vec![UserMessageContent::Text(
            "bar".into()
          ),]))
          .into()
        ])
        .unwrap()
        .into(),
        created_at: 1,
        id: Some(2),
        settings,
        title: Some("bar".into()),
        transcript: vec![TranscriptEntry::Message(Message::User(vec![
          UserMessageContent::Text("bar".into())
        ]))],
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
        max_output_tokens: None,
        max_rounds: AgentConfig::DEFAULT_MAX_ROUNDS,
        model: "mock:local".parse().unwrap(),
        prompt: None,
        yolo: false,
      },
      0,
    ))
    .unwrap();

    state.run = Some(Run::new(0));

    assert_eq!(
      state.handle_event(Event::Error("foo".into())),
      vec![Effect::InterruptAgent]
    );

    assert_eq!(
      state.handle_event(Event::AgentCompleted {
        result: RunState::new(state.session.conversation().clone())
          .finish(RunOutcome::Cancelled),
        run_id: 0,
      }),
      vec![Effect::SaveSession]
    );

    assert_eq!(state.run, None);

    assert_eq!(
      state.session.transcript,
      [TranscriptEntry::Error("foo".into())]
    );
  }

  #[test]
  fn terminal_error_without_run() {
    let mut state = State::new(Session::new(
      &Settings {
        directory: "foo".into(),
        max_output_tokens: None,
        max_rounds: AgentConfig::DEFAULT_MAX_ROUNDS,
        model: "mock:local".parse().unwrap(),
        prompt: None,
        yolo: false,
      },
      0,
    ))
    .unwrap();

    assert_eq!(
      state.handle_event(Event::Error("foo".into())),
      vec![Effect::SaveSession]
    );

    assert_eq!(state.run, None);

    assert_eq!(
      state.session.transcript,
      [TranscriptEntry::Error("foo".into())]
    );
  }
}
