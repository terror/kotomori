use super::*;

#[derive(Debug)]
pub(crate) struct App {
  agent: Agent,
  event_receiver: UnboundedReceiver<Event>,
  event_sender: UnboundedSender<Event>,
  state: State,
}

impl App {
  fn handle_effect(&self, effect: Effect) {
    match effect {
      Effect::RunAgent { input } => {
        self.agent.spawn(input);
      }
    }
  }

  fn handle_event(&mut self, event: Event) {
    if let Some(effect) = self.state.handle_event(event) {
      self.handle_effect(effect);
    }
  }

  pub(crate) fn new(options: Options) -> Self {
    let (event_sender, event_receiver) = mpsc::unbounded_channel();

    Self {
      agent: Agent::new(event_sender.clone(), options.model),
      event_receiver,
      event_sender,
      state: State::new(options.prompt.unwrap_or_default()),
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

  pub(crate) async fn run(mut self) -> Result {
    let mut terminal = Terminal::new()?;

    self.spawn_terminal_events();

    while !self.state.should_quit() {
      terminal.draw(|frame| self.render(frame))?;

      let Some(event) = self.event_receiver.recv().await else {
        break;
      };

      self.handle_event(event);

      while let Ok(event) = self.event_receiver.try_recv() {
        self.handle_event(event);
      }
    }

    Ok(())
  }

  fn spawn_terminal_events(&self) {
    let sender = self.event_sender.clone();

    thread::spawn(move || {
      loop {
        let event = match ratatui::crossterm::event::read() {
          Ok(event) => event,
          Err(error) => {
            let _ = sender.send(Event::Error(error.to_string()));
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

        if sender.send(Event::Action(action)).is_err() {
          return;
        }
      }
    });
  }
}
