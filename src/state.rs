use super::*;

#[derive(Debug)]
pub(crate) struct State {
  composer: Composer,
  should_quit: bool,
  transcript: Transcript,
}

impl State {
  pub(crate) fn composer(&self) -> &Composer {
    &self.composer
  }

  fn handle_action(&mut self, action: Action) -> Vec<Effect> {
    match action {
      Action::CompleteCommand => {
        if !self.composer.complete_command() {
          self.composer.input(Input {
            key: Key::Tab,
            ..Default::default()
          });
        }
      }
      Action::Edit(input) => self.composer.input(input),
      Action::Quit => self.quit(),
      Action::SelectNextCommand => {
        if self.composer.selected_command().is_some() {
          self.composer.select_next_command();
        } else {
          self.composer.input(Input {
            key: Key::Down,
            ..Default::default()
          });
        }
      }
      Action::SelectPreviousCommand => {
        if self.composer.selected_command().is_some() {
          self.composer.select_previous_command();
        } else {
          self.composer.input(Input {
            key: Key::Up,
            ..Default::default()
          });
        }
      }
      Action::Submit => return self.submit(),
    }

    Vec::new()
  }

  pub(crate) fn handle_event(&mut self, event: Event) -> Vec<Effect> {
    match event {
      Event::Action(action) => return self.handle_action(action),
      Event::AgentDone => self.transcript.finish_agent_message(),
      Event::AgentDelta(delta) => self.transcript.push_agent_delta(&delta),
      Event::AgentToolCall(tool_call) => {
        self.transcript.push_tool_call(tool_call);
      }
      Event::AgentToolResult { id, result } => {
        self.transcript.push_tool_result(&id, result);
      }
      Event::Error(error) => self.transcript.error(error),
      Event::Tick => self.transcript.tick(),
    }

    Vec::new()
  }

  #[cfg(test)]
  pub(crate) fn input_text(&self) -> String {
    self.composer.input_text()
  }

  pub(crate) fn new(options: &Options) -> Result<Self> {
    Ok(Self {
      composer: Composer::new(options.prompt.as_deref().unwrap_or_default())
        .footer(Footer::try_from(&options.model)?),
      should_quit: false,
      transcript: Transcript::default(),
    })
  }

  fn quit(&mut self) {
    self.should_quit = true;
  }

  fn reset_input(&mut self) {
    self.composer.clear();
  }

  fn run_command(&mut self, command: Command) -> Vec<Effect> {
    match command {
      Command::Clear => self.transcript.clear(),
      Command::Quit => self.quit(),
    }

    self.reset_input();

    Vec::new()
  }

  pub(crate) fn should_quit(&self) -> bool {
    self.should_quit
  }

  fn submit(&mut self) -> Vec<Effect> {
    let input = self.composer.input_text();
    let input = input.trim();

    if let Some(command) = Command::from_input(input) {
      return self.run_command(command);
    }

    if input.starts_with('/') {
      let command = self.composer.selected_command();

      if let Some(command) = command {
        self.run_command(command);
      } else if input.len() > 1 {
        self.transcript.push_agent(format!(
          "Unrecognized command '{input}'. Type \"/\" for a list of supported commands."
        ));

        self.reset_input();
      }

      return Vec::new();
    }

    if self.transcript.is_agent_active() {
      return Vec::new();
    }

    if input.is_empty() {
      return Vec::new();
    }

    let input = input.to_string();

    self.transcript.send(input.clone());

    let messages = self.transcript.messages();

    self.reset_input();

    vec![Effect::RunAgent { messages }]
  }

  pub(crate) fn transcript(&self) -> &Transcript {
    &self.transcript
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn active_frame_ticks() {
    let mut state = State::new(&Options {
      model: "fake:local".parse().unwrap(),
      prompt: Some("foo".into()),
    })
    .unwrap();

    state.handle_event(Event::Action(Action::Submit));

    assert!(
      state.transcript().render(80).ends_with(&[
        Line::blank(),
        vec![
          Span::styled("✦", Style::CyanBold),
          Span::styled(" Working...", Style::Gray),
        ]
        .into(),
        Line::blank(),
      ])
    );

    state.handle_event(Event::Tick);

    assert!(
      state.transcript().render(80).ends_with(&[
        Line::blank(),
        vec![
          Span::styled("✧", Style::CyanBold),
          Span::styled(" Working...", Style::Gray),
        ]
        .into(),
        Line::blank(),
      ])
    );

    state.handle_event(Event::AgentDone);
    state.handle_event(Event::Tick);

    assert!(
      !state
        .transcript()
        .render(80)
        .iter()
        .any(|line| line.to_string().contains("Working"))
    );
  }

  #[test]
  fn command_autocomplete() {
    let mut state = State::new(&Options {
      model: "fake:local".parse().unwrap(),
      prompt: Some("/".into()),
    })
    .unwrap();

    assert_eq!(
      state
        .composer()
        .commands()
        .map(Command::name)
        .collect::<Vec<_>>(),
      vec!["clear", "quit"],
    );

    state.handle_event(Event::Action(Action::SelectNextCommand));
    state.handle_event(Event::Action(Action::CompleteCommand));

    assert_eq!(state.input_text(), "/quit");

    state.handle_event(Event::Action(Action::Submit));

    assert!(state.should_quit());
  }

  #[test]
  fn command_clear() {
    #[track_caller]
    fn case(command: &str) {
      let mut state = State::new(&Options {
        model: "fake:local".parse().unwrap(),
        prompt: Some("foo".into()),
      })
      .unwrap();

      assert_eq!(
        state.handle_event(Event::Action(Action::Submit)),
        vec![Effect::RunAgent {
          messages: vec![Message::new(Role::User, "foo")]
        }]
      );

      state.handle_event(Event::AgentDelta("bar".into()));
      state.handle_event(Event::AgentDone);

      for c in command.chars() {
        state.handle_event(Event::Action(Action::Edit(Input {
          key: Key::Char(c),
          ..Default::default()
        })));
      }

      state.handle_event(Event::Action(Action::Submit));

      assert_eq!(state.transcript().messages(), &[]);
      assert_eq!(state.input_text(), "");
    }

    case("/");
    case("/c");
  }

  #[test]
  fn error_clears_active_message() {
    let mut state = State::new(&Options {
      model: "fake:local".parse().unwrap(),
      prompt: Some("foo".into()),
    })
    .unwrap();

    assert_eq!(
      state.handle_event(Event::Action(Action::Submit)),
      vec![Effect::RunAgent {
        messages: vec![Message::new(Role::User, "foo")]
      }]
    );

    state.handle_event(Event::Error("bar".into()));

    assert_eq!(
      state.transcript().messages()[1],
      Message::new(Role::Agent, "bar")
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
          Message::new(Role::User, "foo"),
          Message::new(Role::Agent, "bar"),
          Message::new(Role::User, "baz"),
        ]
      }]
    );
  }

  #[test]
  fn multiline_input() {
    let mut state = State::new(&Options {
      model: "fake:local".parse().unwrap(),
      prompt: Some(String::new()),
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
        messages: vec![Message::new(Role::User, "foo\nbar")]
      }]
    );
  }

  #[test]
  fn unknown_command() {
    let mut state = State::new(&Options {
      model: "fake:local".parse().unwrap(),
      prompt: Some("/foobar".into()),
    })
    .unwrap();

    state.handle_event(Event::Action(Action::Submit));

    assert_eq!(
      state.transcript().messages(),
      &[Message::new(
        Role::Agent,
        "Unrecognized command '/foobar'. Type \"/\" for a list of supported commands."
      )]
    );

    assert_eq!(state.input_text(), "");
  }
}
