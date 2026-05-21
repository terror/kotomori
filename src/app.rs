use super::*;

#[derive(Debug)]
pub(crate) struct App {
  runtime: Runtime,
  state: State,
  stream: UnboundedReceiver<Action>,
  stream_sender: UnboundedSender<Action>,
}

impl App {
  fn handle_action(&mut self, action: Action) {
    if let Some(request) = self.state.handle_action(action) {
      self.spawn_agent(request);
    }
  }

  fn handle_crossterm_event(&mut self, event: &CrosstermEvent) {
    let CrosstermEvent::Key(key) = event else {
      return;
    };

    if key.kind != KeyEventKind::Press {
      return;
    }

    self.handle_action(Action::from(key));
  }

  pub(crate) fn new(options: Options) -> Result<Self> {
    let (stream_sender, stream) = mpsc::unbounded_channel();

    Ok(Self {
      runtime: Runtime::new().context("failed to initialize async runtime")?,
      state: State::new(options),
      stream,
      stream_sender,
    })
  }

  fn receive_stream(&mut self) {
    while let Ok(action) = self.stream.try_recv() {
      self.handle_action(action);
    }
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

    while !self.state.should_quit() {
      self.receive_stream();

      terminal.draw(|frame| self.render(frame))?;

      let timeout = if self.state.is_agent_active() {
        Duration::from_millis(16)
      } else {
        Duration::from_millis(250)
      };

      if event::poll(timeout)? {
        self.handle_crossterm_event(&event::read()?);
      }
    }

    Ok(())
  }

  fn spawn_agent(&self, request: AgentRequest) {
    let sender = self.stream_sender.clone();

    self.runtime.spawn(async move {
      let response = format!("queued for {}: {}", request.model, request.input);

      for c in response.chars() {
        if sender.send(Action::AgentOutput(c)).is_err() {
          return;
        }

        tokio::time::sleep(Duration::from_millis(20)).await;
      }

      let _ = sender.send(Action::AgentDone);
    });
  }
}
