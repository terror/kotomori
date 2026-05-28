use super::*;

#[derive(Debug)]
pub(crate) struct ResumePicker {
  query: String,
  selected: usize,
  sessions: Vec<SessionSummary>,
}

impl ResumePicker {
  fn clamp_selection(&mut self) {
    let len = self.filtered_len();

    self.selected = if len == 0 {
      0
    } else {
      self.selected.min(len.saturating_sub(1))
    };
  }

  fn filtered(&self) -> Vec<&SessionSummary> {
    self
      .sessions
      .iter()
      .filter(|session| session.matches(&self.query))
      .collect()
  }

  fn filtered_len(&self) -> usize {
    self
      .sessions
      .iter()
      .filter(|session| session.matches(&self.query))
      .count()
  }

  fn handle_key(&mut self, key: KeyEvent) -> Option<ResumePickerAction> {
    match key.code {
      KeyCode::Backspace => {
        self.query.pop();
        self.clamp_selection();
      }
      KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
        return Some(ResumePickerAction::Cancel);
      }
      KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
        self.query.clear();
        self.clamp_selection();
      }
      KeyCode::Char(c)
        if !key
          .modifiers
          .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
      {
        self.query.push(c);
        self.clamp_selection();
      }
      KeyCode::Down => {
        let len = self.filtered_len();

        if len > 0 {
          self.selected = self.selected.saturating_add(1) % len;
        }
      }
      KeyCode::Enter => {
        if let Some(path) = self.selected_path() {
          return Some(ResumePickerAction::Resume(path));
        }
      }
      KeyCode::Esc => return Some(ResumePickerAction::Cancel),
      KeyCode::Up => {
        let len = self.filtered_len();

        if len > 0 {
          self.selected = if self.selected == 0 {
            len.saturating_sub(1)
          } else {
            self.selected.saturating_sub(1)
          };
        }
      }
      _ => {}
    }

    None
  }

  pub(crate) fn new(sessions: Vec<SessionSummary>) -> Self {
    Self {
      query: String::new(),
      selected: 0,
      sessions,
    }
  }

  pub(crate) fn run(mut self) -> Result<Option<PathBuf>> {
    let mut terminal = Terminal::new()?;

    let mut renderer = Renderer::new();

    loop {
      renderer.draw(&mut terminal.stdout, &self)?;

      let event =
        crossterm_event::read().context("failed to read terminal input")?;

      let CrosstermEvent::Key(key) = event else {
        continue;
      };

      if key.kind != KeyEventKind::Press {
        continue;
      }

      if let Some(action) = self.handle_key(key) {
        renderer.finish(&mut terminal.stdout)?;

        return Ok(match action {
          ResumePickerAction::Cancel => None,
          ResumePickerAction::Resume(path) => Some(path),
        });
      }
    }
  }

  fn selected_path(&self) -> Option<PathBuf> {
    self
      .filtered()
      .get(self.selected)
      .map(|session| session.path.clone())
  }
}

impl Component for ResumePicker {
  fn render(&self, width: u16) -> Vec<Line> {
    let mut lines = once(Line::blank())
      .chain(Header.render(width))
      .chain(once(Line::blank()))
      .chain(once(Line::from([
        Span::styled("Search previous sessions. Press ", Style::DarkGray),
        Span::styled("Enter", Style::Gray),
        Span::styled(" to resume, ", Style::DarkGray),
        Span::styled("Esc", Style::Gray),
        Span::styled(" to cancel.", Style::DarkGray),
      ])))
      .chain(once(Line::blank()))
      .chain(once(Line::from([
        Span::styled("Search: ", Style::DarkGray),
        Span::raw(&self.query),
        Span::styled(" ", Style::Reverse),
      ])))
      .chain(once(Line::blank()))
      .collect::<Vec<_>>();

    let filtered = self.filtered();

    if filtered.is_empty() {
      lines.push(Line::from([Span::styled(
        "No matching sessions.",
        Style::DarkGray,
      )]));

      return lines;
    }

    for (index, session) in filtered.into_iter().enumerate() {
      let style = if index == self.selected {
        Style::CyanBold
      } else {
        Style::Gray
      };

      let marker = if index == self.selected { "> " } else { "  " };

      lines.push(Line::from([
        Span::styled(marker, style),
        Span::styled(session.title.as_str(), style),
        Span::styled("  ", Style::DarkGray),
        Span::styled(session.detail(), Style::DarkGray),
      ]));
    }

    lines
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn filters_sessions() {
    let mut picker = ResumePicker::new(vec![
      SessionSummary {
        cwd: "foo".into(),
        model: "mock:local".into(),
        path: "foo".into(),
        search: "foo".into(),
        title: "foo".into(),
        updated_at: 0,
      },
      SessionSummary {
        cwd: "bar".into(),
        model: "mock:local".into(),
        path: "bar".into(),
        search: "bar".into(),
        title: "bar".into(),
        updated_at: 0,
      },
    ]);

    picker.handle_key(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::NONE));

    assert!(
      picker
        .render(80)
        .iter()
        .any(|line| { line.to_string().contains("bar") })
    );

    assert!(
      !picker
        .render(80)
        .iter()
        .any(|line| { line.to_string().contains("foo  mock") })
    );
  }
}
