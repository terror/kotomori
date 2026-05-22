use super::*;

#[derive(Debug)]
pub(crate) struct App {
  event_receiver: UnboundedReceiver<AppEvent>,
  event_sender: UnboundedSender<AppEvent>,
  model: String,
  runtime: Runtime,
  state: State,
}

impl App {
  fn handle_action(&mut self, action: Action) {
    if let Some(effect) = self.state.handle_action(action) {
      self.handle_effect(effect);
    }
  }

  fn handle_effect(&self, effect: Effect) {
    match effect {
      Effect::RunAgent { input } => self.spawn_agent(input),
    }
  }

  pub(crate) fn new(options: Options) -> Result<Self> {
    let (event_sender, event_receiver) = mpsc::unbounded_channel();

    Ok(Self {
      event_receiver,
      event_sender,
      model: options.model,
      runtime: Runtime::new().context("failed to initialize async runtime")?,
      state: State::new(options.prompt.unwrap_or_default()),
    })
  }

  fn render(&self, frame: &mut Frame) {
    let area = frame.area();

    let transcript_height = self
      .state
      .transcript_height(area.width)
      .min(area.height.saturating_sub(6));

    let [
      _,
      header_area,
      _,
      hint_area,
      _,
      transcript_area,
      composer,
      _,
    ] = Layout::default()
      .direction(Direction::Vertical)
      .constraints([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(transcript_height),
        Constraint::Length(1),
        Constraint::Min(0),
      ])
      .areas(area);

    let header = Paragraph::new(Line::from(vec![
      Span::raw("  "),
      Span::styled(
        "kotomori",
        Style::default()
          .fg(Color::Cyan)
          .add_modifier(Modifier::BOLD),
      ),
      Span::raw("  "),
      Span::styled(
        env!("CARGO_PKG_VERSION"),
        Style::default().fg(Color::DarkGray),
      ),
    ]));

    frame.render_widget(header, header_area);

    let hint = Paragraph::new(Line::from(vec![
      Span::styled(
        "  Type a prompt. Press ",
        Style::default().fg(Color::DarkGray),
      ),
      Span::styled("Ctrl-C", Style::default().fg(Color::Gray)),
      Span::styled(" to quit.", Style::default().fg(Color::DarkGray)),
    ]));

    frame.render_widget(hint, hint_area);

    let lines = self
      .state
      .messages()
      .iter()
      .flat_map(Message::lines)
      .collect::<Vec<_>>();

    let transcript = Paragraph::new(lines).wrap(Wrap { trim: false });

    frame.render_widget(transcript, transcript_area);

    let input = Paragraph::new(Line::from(vec![
      Span::raw("  "),
      Span::styled(
        "❯ ",
        Style::default()
          .fg(Color::Cyan)
          .add_modifier(Modifier::BOLD),
      ),
      Span::raw(self.state.input()),
    ]));

    frame.render_widget(input, composer);

    let input_len = u16::try_from(self.state.input().len()).unwrap_or(u16::MAX);

    frame.set_cursor_position((
      composer.x.saturating_add(input_len).saturating_add(4),
      composer.y,
    ));
  }

  pub(crate) fn run(mut self) -> Result {
    let mut terminal = Terminal::new()?;

    self.spawn_terminal_events();

    while !self.state.should_quit() {
      terminal.draw(|frame| self.render(frame))?;

      let Some(event) = self.event_receiver.blocking_recv() else {
        break;
      };

      self.handle_action(event?);

      while let Ok(event) = self.event_receiver.try_recv() {
        self.handle_action(event?);
      }
    }

    Ok(())
  }

  fn spawn_agent(&self, input: String) {
    let model = self.model.clone();

    let sender = self.event_sender.clone();

    self.runtime.spawn(async move {
      let response = format!("queued for {model}: {input}");

      for c in response.chars() {
        if sender.send(Ok(Action::AgentOutput(c))).is_err() {
          return;
        }

        tokio::time::sleep(Duration::from_millis(20)).await;
      }

      let _ = sender.send(Ok(Action::AgentDone));
    });
  }

  fn spawn_terminal_events(&self) {
    let sender = self.event_sender.clone();

    thread::spawn(move || {
      loop {
        let event = match event::read() {
          Ok(event) => event,
          Err(error) => {
            let _ = sender.send(Err(error));
            return;
          }
        };

        let CrosstermEvent::Key(key) = event else {
          continue;
        };

        if key.kind != KeyEventKind::Press {
          continue;
        }

        let Some(action) = Action::from_key(&key) else {
          continue;
        };

        if sender.send(Ok(action)).is_err() {
          return;
        }
      }
    });
  }
}
