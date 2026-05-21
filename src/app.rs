use super::*;

#[derive(Debug)]
pub(crate) struct App {
  state: State,
}

impl App {
  fn handle_crossterm_event(&mut self, event: &CrosstermEvent) {
    let CrosstermEvent::Key(key) = event else {
      return;
    };

    if key.kind != KeyEventKind::Press {
      return;
    }

    self.state.handle_action(Action::from(key));
  }

  pub(crate) fn new(options: Options) -> Self {
    Self {
      state: State::new(options),
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
      terminal.draw(|frame| self.render(frame))?;

      if event::poll(Duration::from_millis(250))? {
        self.handle_crossterm_event(&event::read()?);
      }
    }

    Ok(())
  }
}
