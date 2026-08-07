use super::*;

pub(crate) trait WriteExt {
  fn begin_synchronized_update(&mut self) -> Result;
  fn clear_line(&mut self) -> Result;
  fn clear_screen(&mut self) -> Result;
  fn end_synchronized_update(&mut self) -> Result;
  fn move_down(&mut self, lines: usize) -> Result;
  fn move_up(&mut self, lines: usize) -> Result;
  fn write_line(&mut self, line: &str) -> Result;
  fn write_lines(&mut self, lines: &[String]) -> Result;
}

impl<T: Write> WriteExt for T {
  fn begin_synchronized_update(&mut self) -> Result {
    queue!(self, BeginSynchronizedUpdate)?;

    Ok(())
  }

  fn clear_line(&mut self) -> Result {
    queue!(self, Clear(ClearType::CurrentLine))?;

    Ok(())
  }

  fn clear_screen(&mut self) -> Result {
    queue!(
      self,
      Clear(ClearType::All),
      MoveTo(0, 0),
      Clear(ClearType::Purge)
    )?;

    Ok(())
  }

  fn end_synchronized_update(&mut self) -> Result {
    queue!(self, EndSynchronizedUpdate)?;

    Ok(())
  }

  fn move_down(&mut self, lines: usize) -> Result {
    if lines > 0 {
      queue!(self, MoveDown(u16::try_from(lines).unwrap_or(u16::MAX)))?;
    }

    Ok(())
  }

  fn move_up(&mut self, lines: usize) -> Result {
    if lines > 0 {
      queue!(self, MoveUp(u16::try_from(lines).unwrap_or(u16::MAX)))?;
    }

    Ok(())
  }

  fn write_line(&mut self, line: &str) -> Result {
    self.clear_line()?;

    write!(self, "{line}")?;

    Ok(())
  }

  fn write_lines(&mut self, lines: &[String]) -> Result {
    for (index, line) in lines.iter().enumerate() {
      if index > 0 {
        write!(self, "\r\n")?;
      }

      queue!(self, MoveToColumn(0))?;

      self.write_line(line)?;
    }

    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn begin_synchronized_update_writes_sequence() {
    let mut stdout = Vec::new();

    stdout.begin_synchronized_update().unwrap();

    assert_eq!(String::from_utf8(stdout).unwrap(), "\x1b[?2026h");
  }

  #[test]
  fn clear_line_writes_sequence() {
    let mut stdout = Vec::new();

    stdout.clear_line().unwrap();

    assert_eq!(String::from_utf8(stdout).unwrap(), "\x1b[2K");
  }

  #[test]
  fn clear_screen_writes_sequence() {
    let mut stdout = Vec::new();

    stdout.clear_screen().unwrap();

    assert_eq!(
      String::from_utf8(stdout).unwrap(),
      "\x1b[2J\x1b[1;1H\x1b[3J"
    );
  }

  #[test]
  fn end_synchronized_update_writes_sequence() {
    let mut stdout = Vec::new();

    stdout.end_synchronized_update().unwrap();

    assert_eq!(String::from_utf8(stdout).unwrap(), "\x1b[?2026l");
  }

  #[test]
  fn write_line_clears_and_writes_line() {
    let mut stdout = Vec::new();

    stdout.write_line("foo").unwrap();

    assert_eq!(String::from_utf8(stdout).unwrap(), "\x1b[2Kfoo");
  }

  #[test]
  fn write_lines_scrolls() {
    let mut stdout = Vec::new();

    stdout.write_lines(&["foo".into(), "bar".into()]).unwrap();

    assert_eq!(
      String::from_utf8(stdout).unwrap(),
      "\x1b[1G\x1b[2Kfoo\r\n\x1b[1G\x1b[2Kbar",
    );
  }
}
