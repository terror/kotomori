use super::*;

pub(crate) trait WriteExt {
  /// Begin a synchronized terminal update.
  ///
  /// This queues crossterm's `BeginSynchronizedUpdate` command, which asks
  /// compatible terminal emulators to keep presenting the previous visible
  /// state while subsequent bytes update the terminal buffer. The command is
  /// intentionally not flushed here, so callers can batch it with the writes
  /// that follow and eventually pair it with `end_synchronized_update`.
  fn begin_synchronized_update(&mut self) -> Result;

  /// Clear the terminal row under the cursor.
  ///
  /// This queues a crossterm `Clear` command for `ClearType::CurrentLine`
  /// without changing the cursor position. Callers that need to clear from the
  /// beginning of a row should move the cursor to column zero before calling
  /// this method.
  fn clear_line(&mut self) -> Result;

  /// Clear the terminal for a full redraw.
  ///
  /// This clears the visible screen, moves the cursor to the top-left corner,
  /// and purges the scrollback buffer. The three commands are queued together
  /// so a renderer can prepare a clean frame before writing replacement lines.
  fn clear_screen(&mut self) -> Result;

  /// End a synchronized terminal update.
  ///
  /// This queues crossterm's `EndSynchronizedUpdate` command, which lets
  /// compatible terminal emulators present all terminal-buffer changes made
  /// since `begin_synchronized_update`. The command is intentionally not
  /// flushed here, leaving flushing policy with the caller.
  fn end_synchronized_update(&mut self) -> Result;

  /// Move the cursor vertically by a signed line delta.
  ///
  /// Positive values move the cursor down, negative values move it up, and
  /// zero leaves the stream unchanged. Large absolute values are delegated to
  /// `move_up` or `move_down`, which clamp the terminal command count to the
  /// largest `u16` value crossterm accepts.
  fn move_by(&mut self, diff: isize) -> Result;

  /// Move the cursor down by `lines` terminal rows.
  ///
  /// A zero count writes nothing. Nonzero counts queue a crossterm `MoveDown`
  /// command, with counts larger than `u16::MAX` clamped to `u16::MAX`.
  fn move_down(&mut self, lines: usize) -> Result;

  /// Move the cursor up by `lines` terminal rows.
  ///
  /// A zero count writes nothing. Nonzero counts queue a crossterm `MoveUp`
  /// command, with counts larger than `u16::MAX` clamped to `u16::MAX`.
  fn move_up(&mut self, lines: usize) -> Result;

  /// Write one rendered frame line at the current terminal position.
  ///
  /// The current terminal line is cleared before the provided string is
  /// written. The cursor column is otherwise left to the caller, which allows
  /// patch rendering to use carriage returns and full rendering to use
  /// `MoveToColumn(0)` while sharing the same clear-and-write behavior. The
  /// string is expected to already contain any style reset sequences needed by
  /// the caller.
  fn write_line(&mut self, line: &str) -> Result;

  /// Write rendered frame lines at the current terminal position.
  ///
  /// Each line starts at column zero, clears the current terminal line, and
  /// then writes the provided string. Lines after the first are preceded by
  /// `\r\n`, which lets the terminal scroll naturally when writing reaches the
  /// bottom of the viewport. The strings are expected to already contain any
  /// style reset sequences needed by the caller.
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

  fn move_by(&mut self, diff: isize) -> Result {
    match diff.cmp(&0) {
      Ordering::Less => self.move_up(diff.unsigned_abs())?,
      Ordering::Equal => {}
      Ordering::Greater => self.move_down(diff.unsigned_abs())?,
    }

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
  fn move_by_moves_down_for_positive_diff() {
    let mut stdout = Vec::new();

    stdout.move_by(2).unwrap();

    assert_eq!(String::from_utf8(stdout).unwrap(), "\x1b[2B");
  }

  #[test]
  fn move_by_moves_up_for_negative_diff() {
    let mut stdout = Vec::new();

    stdout.move_by(-2).unwrap();

    assert_eq!(String::from_utf8(stdout).unwrap(), "\x1b[2A");
  }

  #[test]
  fn move_by_writes_nothing_for_zero_diff() {
    let mut stdout = Vec::new();

    stdout.move_by(0).unwrap();

    assert_eq!(stdout, Vec::<u8>::new());
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
