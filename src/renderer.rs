use super::*;

#[derive(Debug)]
pub(crate) struct Renderer {
  previous: Vec<String>,
  previous_width: u16,
}

impl Renderer {
  fn append_lines(
    &mut self,
    stdout: &mut impl Write,
    lines: &[String],
  ) -> Result {
    if lines.is_empty() {
      return Ok(());
    }

    if self.previous.is_empty() {
      queue!(stdout, MoveToColumn(0))?;
    } else {
      write!(stdout, "\r\n")?;
    }

    Self::write_lines(stdout, lines)?;

    Ok(())
  }

  fn can_redraw(&self, first_changed_line: usize, height: u16) -> bool {
    self
      .previous
      .len()
      .saturating_sub(first_changed_line)
      .saturating_sub(1)
      < usize::from(height)
  }

  pub(crate) fn draw(
    &mut self,
    stdout: &mut Stdout,
    component: &impl Component,
  ) -> Result {
    let (width, height) =
      crossterm_terminal::size().context("failed to read terminal size")?;

    let rendered = component
      .render(width)
      .into_iter()
      .flat_map(|line| line.render(width))
      .map(|line| line.to_string())
      .collect::<Vec<_>>();

    write!(stdout, "\x1b[?2026h")?;

    match self.refresh(&rendered, width, height) {
      Refresh::Append { from } => {
        self.append_lines(stdout, &rendered[from..])?;
      }
      Refresh::FullAppend => self.append_lines(stdout, &rendered)?,
      Refresh::Initial => Self::write_lines(stdout, &rendered)?,
      Refresh::RedrawTail { from } => {
        self.redraw_tail(stdout, &rendered, from)?;
      }
    }

    self.previous = rendered;
    self.previous_width = width;

    write!(stdout, "\x1b[?2026l")?;

    stdout.flush()?;

    Ok(())
  }

  fn first_changed_line(&self, rendered: &[String]) -> usize {
    let common = self.previous.len().min(rendered.len());

    self
      .previous
      .iter()
      .zip(rendered)
      .position(|(previous, next)| previous != next)
      .unwrap_or(common)
  }

  fn is_append_only(
    &self,
    rendered: &[String],
    first_changed_line: usize,
  ) -> bool {
    first_changed_line == self.previous.len()
      && rendered.len() >= self.previous.len()
  }

  fn move_to_previous_line(
    &mut self,
    stdout: &mut Stdout,
    line: usize,
  ) -> Result {
    if self.previous.is_empty() {
      return Ok(());
    }

    queue!(stdout, MoveToColumn(0))?;

    let current = self.previous.len().saturating_sub(1);

    match line.cmp(&current) {
      Ordering::Less => queue!(
        stdout,
        MoveUp(u16::try_from(current.saturating_sub(line)).unwrap_or(u16::MAX))
      )?,
      Ordering::Equal => {}
      Ordering::Greater => queue!(
        stdout,
        MoveDown(
          u16::try_from(line.saturating_sub(current)).unwrap_or(u16::MAX)
        )
      )?,
    }

    Ok(())
  }

  pub(crate) fn new() -> Self {
    Self {
      previous: Vec::new(),
      previous_width: 0,
    }
  }

  fn redraw_tail(
    &mut self,
    stdout: &mut Stdout,
    rendered: &[String],
    first_changed_line: usize,
  ) -> Result {
    self.move_to_previous_line(stdout, first_changed_line)?;

    queue!(stdout, Clear(ClearType::FromCursorDown))?;

    if first_changed_line < rendered.len() {
      Self::write_lines(stdout, &rendered[first_changed_line..])?;
    } else if first_changed_line > 0 {
      queue!(stdout, MoveUp(1))?;
    }

    Ok(())
  }

  fn refresh(&self, rendered: &[String], width: u16, height: u16) -> Refresh {
    if self.previous.is_empty() {
      return Refresh::Initial;
    }

    let first_changed_line = if self.previous_width == width {
      self.first_changed_line(rendered)
    } else {
      0
    };

    if self.is_append_only(rendered, first_changed_line) {
      Refresh::Append {
        from: first_changed_line,
      }
    } else if self.can_redraw(first_changed_line, height) {
      Refresh::RedrawTail {
        from: first_changed_line,
      }
    } else {
      Refresh::FullAppend
    }
  }

  fn write_lines(stdout: &mut impl Write, lines: &[String]) -> Result {
    for (index, line) in lines.iter().enumerate() {
      queue!(stdout, MoveToColumn(0), Clear(ClearType::CurrentLine))?;

      write!(stdout, "{line}")?;

      if index + 1 < lines.len() {
        write!(stdout, "\r\n")?;
      }
    }

    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn appending_lines_scrolls() {
    let mut subject = Renderer {
      previous: vec!["foo".into()],
      previous_width: 80,
    };

    let mut stdout = Vec::new();

    subject.append_lines(&mut stdout, &["bar".into()]).unwrap();

    assert_eq!(String::from_utf8(stdout).unwrap(), "\r\n\x1b[1G\x1b[2Kbar");
  }

  #[test]
  fn initially_refreshes_with_initial() {
    let subject = Renderer {
      previous: Vec::new(),
      previous_width: 0,
    };

    assert_eq!(subject.refresh(&["foo".into()], 80, 24), Refresh::Initial);
  }

  #[test]
  fn refreshes_with_append_when_render_extends_previous() {
    let subject = Renderer {
      previous: vec!["foo".into()],
      previous_width: 80,
    };

    assert_eq!(
      subject.refresh(&["foo".into(), "bar".into()], 80, 24),
      Refresh::Append { from: 1 },
    );
  }

  #[test]
  fn refreshes_with_redraw_tail_when_line_changes() {
    let subject = Renderer {
      previous: vec!["foo".into(), "bar".into()],
      previous_width: 80,
    };

    assert_eq!(
      subject.refresh(&["foo".into(), "baz".into()], 80, 24),
      Refresh::RedrawTail { from: 1 },
    );
  }

  #[test]
  fn refreshes_with_full_append_when_tail_cannot_be_redrawn() {
    let subject = Renderer {
      previous: vec!["foo".into(), "bar".into(), "baz".into()],
      previous_width: 80,
    };

    assert_eq!(subject.refresh(&["foo".into()], 80, 0), Refresh::FullAppend);
  }

  #[test]
  fn refreshes_with_redraw_tail_when_width_changes() {
    let subject = Renderer {
      previous: vec!["foo".into()],
      previous_width: 80,
    };

    assert_eq!(
      subject.refresh(&["foo".into(), "bar".into()], 81, 24),
      Refresh::RedrawTail { from: 0 },
    );
  }

  #[test]
  fn writing_lines_scrolls() {
    let mut stdout = Vec::new();

    Renderer::write_lines(&mut stdout, &["foo".into(), "bar".into()]).unwrap();

    assert_eq!(
      String::from_utf8(stdout).unwrap(),
      "\x1b[1G\x1b[2Kfoo\r\n\x1b[1G\x1b[2Kbar",
    );
  }
}
