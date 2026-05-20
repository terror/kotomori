use super::*;

#[derive(Debug)]
pub(crate) struct App {
  input: String,
  messages: Vec<Message>,
  options: Options,
  should_quit: bool,
}

impl App {
  fn handle_crossterm_event(&mut self, event: &CrosstermEvent) {
    let CrosstermEvent::Key(key) = event else {
      return;
    };

    if key.kind != KeyEventKind::Press {
      return;
    }

    self.handle_key(key);
  }

  fn handle_key(&mut self, key: &KeyEvent) {
    match key.code {
      KeyCode::Char('q') if self.input.is_empty() => self.should_quit = true,
      KeyCode::Esc => self.should_quit = true,
      KeyCode::Enter => self.submit(),
      KeyCode::Backspace => {
        self.input.pop();
      }
      KeyCode::Char(c) => self.input.push(c),
      _ => {}
    }
  }

  pub(crate) fn new(options: Options) -> Self {
    let input = options.prompt.clone().unwrap_or_default();

    Self {
      input,
      messages: Vec::new(),
      options,
      should_quit: false,
    }
  }

  fn render(&self, frame: &mut Frame) {
    let area = frame.area();

    let transcript_height = self
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
        "  Type a prompt. Type ",
        Style::default().fg(Color::DarkGray),
      ),
      Span::styled("q", Style::default().fg(Color::Gray)),
      Span::styled(" to quit.", Style::default().fg(Color::DarkGray)),
    ]));

    frame.render_widget(hint, hint_area);

    let lines = self
      .messages
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
      Span::raw(self.input.as_str()),
    ]));

    frame.render_widget(input, composer);

    let input_len = u16::try_from(self.input.len()).unwrap_or(u16::MAX);

    frame.set_cursor_position((
      composer.x.saturating_add(input_len).saturating_add(4),
      composer.y,
    ));
  }

  pub(crate) fn run(mut self) -> Result {
    let mut terminal = Terminal::new()?;

    while !self.should_quit {
      terminal.draw(|frame| self.render(frame))?;

      if event::poll(Duration::from_millis(250))? {
        self.handle_crossterm_event(&event::read()?);
      }
    }

    Ok(())
  }

  fn submit(&mut self) {
    let input = self.input.trim();

    if input.is_empty() {
      return;
    }

    let input = input.to_string();

    self.messages.push(Message::new(Role::User, input.clone()));

    self.messages.push(Message::new(
      Role::Agent,
      format!("queued for {}: {input}", self.options.model),
    ));

    self.input.clear();
  }

  fn transcript_height(&self, width: u16) -> u16 {
    let width = usize::from(width.max(1));

    self
      .messages
      .iter()
      .map(|message| {
        let len = message.width();

        u16::try_from(len.div_ceil(width).max(1)).unwrap_or(u16::MAX)
      })
      .fold(0u16, u16::saturating_add)
  }
}
