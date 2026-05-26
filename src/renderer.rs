use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ChangedRange {
  first: usize,
  last: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Dimensions {
  height: u16,
  width: u16,
}

#[derive(Debug)]
pub(crate) struct Renderer {
  hardware_cursor_row: usize,
  max_lines_rendered: usize,
  previous: Vec<String>,
  previous_height: u16,
  previous_viewport_top: usize,
  previous_width: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Viewport {
  hardware_cursor_row: usize,
  previous_top: usize,
  top: usize,
}

impl Renderer {
  fn changed_range(
    previous: &[String],
    rendered: &[String],
  ) -> Option<ChangedRange> {
    let mut changed = None;

    for index in 0..previous.len().max(rendered.len()) {
      let previous = previous.get(index).map_or("", String::as_str);
      let rendered = rendered.get(index).map_or("", String::as_str);

      if previous != rendered {
        changed = Some(match changed {
          Some(ChangedRange { first, .. }) => {
            ChangedRange { first, last: index }
          }
          None => ChangedRange {
            first: index,
            last: index,
          },
        });
      }
    }

    changed
  }

  fn clear_deleted_lines(
    &mut self,
    stdout: &mut impl Write,
    rendered: Vec<String>,
    dimensions: Dimensions,
    previous_viewport_top: usize,
    changed: ChangedRange,
  ) -> Result {
    let target_row = rendered.len().saturating_sub(1);

    if target_row < previous_viewport_top && !rendered.is_empty() {
      self.full_render(stdout, rendered, dimensions, true)?;
      return Ok(());
    }

    let extra_lines = self.previous.len().saturating_sub(rendered.len());

    if extra_lines > usize::from(dimensions.height) {
      self.full_render(stdout, rendered, dimensions, true)?;
      return Ok(());
    }

    write!(stdout, "\x1b[?2026h")?;

    Self::move_by(
      stdout,
      Self::line_diff(
        self.hardware_cursor_row,
        previous_viewport_top,
        target_row,
        previous_viewport_top,
      ),
    )?;

    write!(stdout, "\r")?;

    if extra_lines > 0 {
      Self::move_down(stdout, 1)?;
    }

    for index in changed.first..=changed.last {
      if index > changed.first {
        Self::move_down(stdout, 1)?;
      }

      write!(stdout, "\r")?;
      queue!(stdout, Clear(ClearType::CurrentLine))?;
    }

    Self::move_up(stdout, extra_lines)?;

    write!(stdout, "\x1b[?2026l")?;

    self.hardware_cursor_row = target_row;
    self.max_lines_rendered = self.max_lines_rendered.max(rendered.len());
    self.previous = rendered;
    self.previous_height = dimensions.height;
    self.previous_viewport_top = previous_viewport_top;
    self.previous_width = dimensions.width;

    Ok(())
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
      .map(|line| format!("{line}{}", Style::None.sequence()))
      .collect::<Vec<_>>();

    self.draw_rendered(stdout, rendered, width, height)?;

    stdout.flush()?;

    Ok(())
  }

  fn draw_changed_lines(
    &mut self,
    stdout: &mut impl Write,
    rendered: Vec<String>,
    dimensions: Dimensions,
    viewport: Viewport,
    changed: ChangedRange,
  ) -> Result {
    let appended = rendered.len() > self.previous.len()
      && changed.first == self.previous.len();
    let append_start = appended && changed.first > 0;
    let height = usize::from(dimensions.height);
    let previous_viewport_bottom = viewport
      .previous_top
      .saturating_add(height.saturating_sub(1));
    let move_target_row = if append_start {
      changed.first.saturating_sub(1)
    } else {
      changed.first
    };
    let mut hardware_cursor_row = viewport.hardware_cursor_row;
    let mut previous_viewport_top = viewport.previous_top;
    let mut viewport_top = viewport.top;

    write!(stdout, "\x1b[?2026h")?;

    if move_target_row > previous_viewport_bottom {
      let current_screen_row =
        hardware_cursor_row.saturating_sub(previous_viewport_top);
      let move_to_bottom =
        height.saturating_sub(1).saturating_sub(current_screen_row);
      Self::move_down(stdout, move_to_bottom)?;

      let scroll = move_target_row.saturating_sub(previous_viewport_bottom);

      for _ in 0..scroll {
        write!(stdout, "\r\n")?;
      }

      previous_viewport_top = previous_viewport_top.saturating_add(scroll);
      viewport_top = viewport_top.saturating_add(scroll);
      hardware_cursor_row = move_target_row;
    }

    Self::move_by(
      stdout,
      Self::line_diff(
        hardware_cursor_row,
        previous_viewport_top,
        move_target_row,
        viewport_top,
      ),
    )?;

    if append_start {
      write!(stdout, "\r\n")?;
    } else {
      write!(stdout, "\r")?;
    }

    let render_end = changed.last.min(rendered.len().saturating_sub(1));

    for (index, line) in rendered
      .iter()
      .enumerate()
      .take(render_end.saturating_add(1))
      .skip(changed.first)
    {
      if index > changed.first {
        write!(stdout, "\r\n")?;
      }

      queue!(stdout, Clear(ClearType::CurrentLine))?;

      write!(stdout, "{line}")?;
    }

    let mut final_cursor_row = render_end;

    if self.previous.len() > rendered.len() {
      if render_end < rendered.len().saturating_sub(1) {
        let move_down = rendered.len().saturating_sub(1) - render_end;
        Self::move_down(stdout, move_down)?;
        final_cursor_row = rendered.len().saturating_sub(1);
      }

      let extra_lines = self.previous.len().saturating_sub(rendered.len());

      for _ in rendered.len()..self.previous.len() {
        write!(stdout, "\r\n")?;
        queue!(stdout, Clear(ClearType::CurrentLine))?;
      }

      Self::move_up(stdout, extra_lines)?;
    }

    write!(stdout, "\x1b[?2026l")?;

    self.hardware_cursor_row = final_cursor_row;
    self.max_lines_rendered = self.max_lines_rendered.max(rendered.len());
    self.previous = rendered;
    self.previous_height = dimensions.height;
    self.previous_viewport_top = previous_viewport_top
      .max(final_cursor_row.saturating_add(1).saturating_sub(height));
    self.previous_width = dimensions.width;

    Ok(())
  }

  fn draw_rendered(
    &mut self,
    stdout: &mut impl Write,
    rendered: Vec<String>,
    width: u16,
    height: u16,
  ) -> Result {
    let dimensions = Dimensions { height, width };
    let width_changed =
      self.previous_width != 0 && self.previous_width != width;
    let height_changed =
      self.previous_height != 0 && self.previous_height != height;

    let previous_buffer_height = if self.previous_height > 0 {
      self
        .previous_viewport_top
        .saturating_add(usize::from(self.previous_height))
    } else {
      usize::from(height)
    };

    let previous_viewport_top = if height_changed {
      previous_buffer_height.saturating_sub(usize::from(height))
    } else {
      self.previous_viewport_top
    };

    if self.previous.is_empty() && !width_changed && !height_changed {
      self.full_render(stdout, rendered, dimensions, false)?;
      return Ok(());
    }

    if width_changed || height_changed && !Self::is_termux_session() {
      self.full_render(stdout, rendered, dimensions, true)?;
      return Ok(());
    }

    if rendered.len() < self.max_lines_rendered
      && env::var_os("KOTOMORI_CLEAR_ON_SHRINK").is_some()
    {
      self.full_render(stdout, rendered, dimensions, true)?;
      return Ok(());
    }

    let Some(changed) = Self::changed_range(&self.previous, &rendered) else {
      self.previous = rendered;
      self.previous_height = height;
      self.previous_viewport_top = previous_viewport_top;
      self.previous_width = width;
      return Ok(());
    };

    if changed.first >= rendered.len() {
      self.clear_deleted_lines(
        stdout,
        rendered,
        dimensions,
        previous_viewport_top,
        changed,
      )?;
      return Ok(());
    }

    if changed.first < previous_viewport_top {
      self.full_render(stdout, rendered, dimensions, true)?;
      return Ok(());
    }

    self.draw_changed_lines(
      stdout,
      rendered,
      dimensions,
      Viewport {
        hardware_cursor_row: self.hardware_cursor_row,
        previous_top: previous_viewport_top,
        top: previous_viewport_top,
      },
      changed,
    )
  }

  pub(crate) fn finish(&mut self, stdout: &mut impl Write) -> Result {
    if self.previous.is_empty() {
      return Ok(());
    }

    let target_row = self.previous.len().saturating_sub(1);

    Self::move_by(
      stdout,
      isize::try_from(target_row).unwrap_or(isize::MAX)
        - isize::try_from(self.hardware_cursor_row).unwrap_or(isize::MAX),
    )?;

    self.hardware_cursor_row = target_row;

    Ok(())
  }

  fn full_render(
    &mut self,
    stdout: &mut impl Write,
    rendered: Vec<String>,
    dimensions: Dimensions,
    clear: bool,
  ) -> Result {
    write!(stdout, "\x1b[?2026h")?;

    if clear {
      queue!(
        stdout,
        MoveTo(0, 0),
        Clear(ClearType::All),
        Clear(ClearType::Purge)
      )?;
    }

    Self::write_lines(stdout, &rendered)?;

    write!(stdout, "\x1b[?2026l")?;

    self.hardware_cursor_row = rendered.len().saturating_sub(1);
    self.max_lines_rendered = if clear {
      rendered.len()
    } else {
      self.max_lines_rendered.max(rendered.len())
    };
    self.previous = rendered;
    self.previous_height = dimensions.height;
    self.previous_viewport_top = self
      .previous
      .len()
      .max(usize::from(dimensions.height))
      .saturating_sub(usize::from(dimensions.height));
    self.previous_width = dimensions.width;

    Ok(())
  }

  fn is_termux_session() -> bool {
    env::var_os("TERMUX_VERSION").is_some()
  }

  fn line_diff(
    hardware_cursor_row: usize,
    previous_viewport_top: usize,
    target_row: usize,
    viewport_top: usize,
  ) -> isize {
    let current_screen_row =
      hardware_cursor_row.saturating_sub(previous_viewport_top);
    let target_screen_row = target_row.saturating_sub(viewport_top);

    isize::try_from(target_screen_row).unwrap_or(isize::MAX)
      - isize::try_from(current_screen_row).unwrap_or(isize::MAX)
  }

  fn move_by(stdout: &mut impl Write, diff: isize) -> Result {
    match diff.cmp(&0) {
      Ordering::Less => Self::move_up(stdout, diff.unsigned_abs())?,
      Ordering::Equal => {}
      Ordering::Greater => Self::move_down(stdout, diff.unsigned_abs())?,
    }

    Ok(())
  }

  fn move_down(stdout: &mut impl Write, lines: usize) -> Result {
    if lines > 0 {
      queue!(stdout, MoveDown(u16::try_from(lines).unwrap_or(u16::MAX)))?;
    }

    Ok(())
  }

  fn move_up(stdout: &mut impl Write, lines: usize) -> Result {
    if lines > 0 {
      queue!(stdout, MoveUp(u16::try_from(lines).unwrap_or(u16::MAX)))?;
    }

    Ok(())
  }

  pub(crate) fn new() -> Self {
    Self {
      hardware_cursor_row: 0,
      max_lines_rendered: 0,
      previous: Vec::new(),
      previous_height: 0,
      previous_viewport_top: 0,
      previous_width: 0,
    }
  }

  fn write_lines(stdout: &mut impl Write, lines: &[String]) -> Result {
    for (index, line) in lines.iter().enumerate() {
      if index > 0 {
        write!(stdout, "\r\n")?;
      }

      write!(stdout, "{line}")?;
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
      hardware_cursor_row: 0,
      max_lines_rendered: 1,
      previous: vec!["foo".into()],
      previous_height: 24,
      previous_viewport_top: 0,
      previous_width: 80,
    };

    let mut stdout = Vec::new();

    subject
      .draw_rendered(&mut stdout, vec!["foo".into(), "bar".into()], 80, 24)
      .unwrap();

    assert_eq!(
      String::from_utf8(stdout).unwrap(),
      "\x1b[?2026h\r\n\x1b[2Kbar\x1b[?2026l",
    );
  }

  #[test]
  fn changed_range_finds_first_and_last_change() {
    assert_eq!(
      Renderer::changed_range(
        &["foo".into(), "bar".into(), "baz".into()],
        &["foo".into(), "qux".into(), "baz".into(), "bob".into()],
      ),
      Some(ChangedRange { first: 1, last: 3 }),
    );
  }

  #[test]
  fn finishing_moves_cursor_to_last_rendered_line() {
    let mut subject = Renderer {
      hardware_cursor_row: 1,
      max_lines_rendered: 3,
      previous: vec!["foo".into(), "bar".into(), "baz".into()],
      previous_height: 24,
      previous_viewport_top: 0,
      previous_width: 80,
    };

    let mut stdout = Vec::new();

    subject.finish(&mut stdout).unwrap();

    assert_eq!(String::from_utf8(stdout).unwrap(), "\x1b[1B");
    assert_eq!(subject.hardware_cursor_row, 2);
  }

  #[test]
  fn full_render_can_clear_screen_and_scrollback() {
    let mut subject = Renderer {
      hardware_cursor_row: 0,
      max_lines_rendered: 0,
      previous: vec!["foo".into()],
      previous_height: 10,
      previous_viewport_top: 0,
      previous_width: 80,
    };

    let mut stdout = Vec::new();

    subject
      .full_render(
        &mut stdout,
        vec!["bar".into(), "baz".into()],
        Dimensions {
          height: 10,
          width: 80,
        },
        true,
      )
      .unwrap();

    assert_eq!(
      String::from_utf8(stdout).unwrap(),
      "\x1b[?2026h\x1b[1;1H\x1b[2J\x1b[3Jbar\r\nbaz\x1b[?2026l",
    );
  }

  #[test]
  fn line_diff_accounts_for_viewport() {
    assert_eq!(Renderer::line_diff(10, 8, 12, 8), 2);
    assert_eq!(Renderer::line_diff(12, 8, 10, 8), -2);
    assert_eq!(Renderer::line_diff(12, 10, 14, 12), 0);
  }

  #[test]
  fn redraws_only_changed_line() {
    let mut subject = Renderer {
      hardware_cursor_row: 2,
      max_lines_rendered: 3,
      previous: vec!["foo".into(), "bar".into(), "baz".into()],
      previous_height: 24,
      previous_viewport_top: 0,
      previous_width: 80,
    };

    let mut stdout = Vec::new();

    subject
      .draw_rendered(
        &mut stdout,
        vec!["foo".into(), "qux".into(), "baz".into()],
        80,
        24,
      )
      .unwrap();

    assert_eq!(
      String::from_utf8(stdout).unwrap(),
      "\x1b[?2026h\x1b[1A\r\x1b[2Kqux\x1b[?2026l",
    );
  }

  #[test]
  fn redraws_screen_when_changed_line_is_above_viewport() {
    let mut subject = Renderer {
      hardware_cursor_row: 2,
      max_lines_rendered: 3,
      previous: vec!["foo".into(), "bar".into(), "baz".into()],
      previous_height: 2,
      previous_viewport_top: 1,
      previous_width: 80,
    };

    let mut stdout = Vec::new();

    subject
      .draw_rendered(
        &mut stdout,
        vec!["qux".into(), "bar".into(), "baz".into()],
        80,
        2,
      )
      .unwrap();

    assert_eq!(
      String::from_utf8(stdout).unwrap(),
      "\x1b[?2026h\x1b[1;1H\x1b[2J\x1b[3Jqux\r\nbar\r\nbaz\x1b[?2026l",
    );
  }

  #[test]
  fn redraws_screen_when_height_changes() {
    let mut subject = Renderer {
      hardware_cursor_row: 0,
      max_lines_rendered: 1,
      previous: vec!["foo".into()],
      previous_height: 24,
      previous_viewport_top: 0,
      previous_width: 80,
    };

    let mut stdout = Vec::new();

    subject
      .draw_rendered(&mut stdout, vec!["foo".into(), "bar".into()], 80, 25)
      .unwrap();

    assert_eq!(
      String::from_utf8(stdout).unwrap(),
      "\x1b[?2026h\x1b[1;1H\x1b[2J\x1b[3Jfoo\r\nbar\x1b[?2026l",
    );
  }

  #[test]
  fn redraws_screen_when_width_changes() {
    let mut subject = Renderer {
      hardware_cursor_row: 0,
      max_lines_rendered: 1,
      previous: vec!["foo".into()],
      previous_height: 24,
      previous_viewport_top: 0,
      previous_width: 80,
    };

    let mut stdout = Vec::new();

    subject
      .draw_rendered(&mut stdout, vec!["foo".into(), "bar".into()], 81, 24)
      .unwrap();

    assert_eq!(
      String::from_utf8(stdout).unwrap(),
      "\x1b[?2026h\x1b[1;1H\x1b[2J\x1b[3Jfoo\r\nbar\x1b[?2026l",
    );
  }

  #[test]
  fn removes_deleted_tail_lines() {
    let mut subject = Renderer {
      hardware_cursor_row: 2,
      max_lines_rendered: 3,
      previous: vec!["foo".into(), "bar".into(), "baz".into()],
      previous_height: 24,
      previous_viewport_top: 0,
      previous_width: 80,
    };

    let mut stdout = Vec::new();

    subject
      .draw_rendered(&mut stdout, vec!["foo".into()], 80, 24)
      .unwrap();

    assert_eq!(
      String::from_utf8(stdout).unwrap(),
      "\x1b[?2026h\x1b[2A\r\x1b[1B\r\x1b[2K\x1b[1B\r\x1b[2K\x1b[2A\x1b[?2026l",
    );
  }

  #[test]
  fn writing_lines_scrolls() {
    let mut stdout = Vec::new();

    Renderer::write_lines(&mut stdout, &["foo".into(), "bar".into()]).unwrap();

    assert_eq!(String::from_utf8(stdout).unwrap(), "foo\r\nbar");
  }
}
