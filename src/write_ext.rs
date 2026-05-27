use super::*;

pub(crate) trait WriteExt {
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

  fn write_lines(&mut self, lines: &[String]) -> Result {
    for (index, line) in lines.iter().enumerate() {
      if index > 0 {
        write!(self, "\r\n")?;
      }

      queue!(self, MoveToColumn(0), Clear(ClearType::CurrentLine))?;

      write!(self, "{line}")?;
    }

    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

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
  fn write_lines_scrolls() {
    let mut stdout = Vec::new();

    stdout.write_lines(&["foo".into(), "bar".into()]).unwrap();

    assert_eq!(
      String::from_utf8(stdout).unwrap(),
      "\x1b[1G\x1b[2Kfoo\r\n\x1b[1G\x1b[2Kbar",
    );
  }
}
