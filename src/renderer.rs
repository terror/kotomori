use super::*;

#[derive(Debug)]
pub(crate) struct Renderer {
  max_lines_rendered: usize,
  presented: Option<PresentedFrame>,
}

impl Renderer {
  fn clear_deleted_tail(
    &self,
    stdout: &mut impl Write,
    next: &Frame,
    diff: Diff,
    previous_viewport: Viewport,
  ) -> Result<PresentedFrame> {
    let presented = self.presented.as_ref().unwrap();

    let target_row = next.last_row();

    write!(stdout, "\x1b[?2026h")?;

    Self::move_by(
      stdout,
      presented.cursor.diff_to(
        previous_viewport,
        target_row,
        previous_viewport,
      ),
    )?;

    write!(stdout, "\r")?;

    if diff.deleted_tail_len() > 0 {
      Self::move_down(stdout, 1)?;
    }

    for index in diff.changed.first..=diff.changed.last {
      if index > diff.changed.first {
        Self::move_down(stdout, 1)?;
      }

      write!(stdout, "\r")?;

      queue!(stdout, Clear(ClearType::CurrentLine))?;
    }

    Self::move_up(stdout, diff.deleted_tail_len())?;

    write!(stdout, "\x1b[?2026l")?;

    Ok(PresentedFrame::new(
      Cursor::new(target_row),
      next.clone(),
      previous_viewport,
    ))
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

  fn draw_rendered(
    &mut self,
    stdout: &mut impl Write,
    rendered: Vec<String>,
    width: u16,
    height: u16,
  ) -> Result {
    let next = Frame::new(rendered, Dimensions { height, width });

    let op = self.render_op(&next);

    let presented = match op {
      RenderOp::Full { clear } => Self::full_render(stdout, &next, clear)?,
      RenderOp::Noop => self.present_noop(&next),
      RenderOp::Patch { diff } => self.patch_render(stdout, next, diff)?,
    };

    self.max_lines_rendered = if matches!(op, RenderOp::Full { clear: true }) {
      presented.frame.len()
    } else {
      self.max_lines_rendered.max(presented.frame.len())
    };

    self.presented = Some(presented);

    Ok(())
  }

  pub(crate) fn finish(&mut self, stdout: &mut impl Write) -> Result {
    let Some(presented) = &self.presented else {
      return Ok(());
    };

    Self::move_by(
      stdout,
      presented.cursor.diff_to(
        presented.viewport,
        presented.frame.last_row(),
        presented.viewport,
      ),
    )?;

    if let Some(presented) = &mut self.presented {
      presented.cursor = Cursor::new(presented.frame.last_row());
    }

    Ok(())
  }

  fn full_render(
    stdout: &mut impl Write,
    next: &Frame,
    clear: bool,
  ) -> Result<PresentedFrame> {
    write!(stdout, "\x1b[?2026h")?;

    if clear {
      queue!(
        stdout,
        MoveTo(0, 0),
        Clear(ClearType::All),
        Clear(ClearType::Purge)
      )?;
    }

    Self::write_lines(stdout, &next.lines)?;

    write!(stdout, "\x1b[?2026l")?;

    Ok(next.clone().into())
  }

  fn is_termux_session() -> bool {
    env::var_os("TERMUX_VERSION").is_some()
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
      max_lines_rendered: 0,
      presented: None,
    }
  }

  fn patch_render(
    &self,
    stdout: &mut impl Write,
    next: Frame,
    diff: Diff,
  ) -> Result<PresentedFrame> {
    let previous_viewport = self.previous_viewport(&next);

    if diff.is_pure_tail_delete() {
      return self.clear_deleted_tail(stdout, &next, diff, previous_viewport);
    }

    let presented = self.presented.as_ref().unwrap();

    let writable_range = diff.writable_range().unwrap();

    let append_start = next.len() > presented.frame.len()
      && diff.changed.first == presented.frame.len();

    let append_start = append_start && diff.changed.first > 0;

    let move_target_row = if append_start {
      diff.changed.first.saturating_sub(1)
    } else {
      diff.changed.first
    };

    let mut cursor = presented.cursor;
    let mut next_viewport = previous_viewport;
    let mut previous_viewport = previous_viewport;

    write!(stdout, "\x1b[?2026h")?;

    if move_target_row > previous_viewport.bottom() {
      let move_to_bottom = previous_viewport
        .height()
        .saturating_sub(1)
        .saturating_sub(previous_viewport.screen_row(cursor.row()));

      Self::move_down(stdout, move_to_bottom)?;

      let scroll = move_target_row.saturating_sub(previous_viewport.bottom());

      for _ in 0..scroll {
        write!(stdout, "\r\n")?;
      }

      cursor = Cursor::new(move_target_row);
      next_viewport = next_viewport.scrolled_down(scroll);
      previous_viewport = previous_viewport.scrolled_down(scroll);
    }

    Self::move_by(
      stdout,
      cursor.diff_to(previous_viewport, move_target_row, next_viewport),
    )?;

    if append_start {
      write!(stdout, "\r\n")?;
    } else {
      write!(stdout, "\r")?;
    }

    for (index, line) in next
      .lines
      .iter()
      .enumerate()
      .take(writable_range.end().saturating_add(1))
      .skip(*writable_range.start())
    {
      if index > *writable_range.start() {
        write!(stdout, "\r\n")?;
      }

      queue!(stdout, Clear(ClearType::CurrentLine))?;

      write!(stdout, "{line}")?;
    }

    let mut cursor = Cursor::new(*writable_range.end());

    if diff.deleted_tail_len() > 0 {
      if cursor.row() < next.last_row() {
        let move_down = next.last_row() - cursor.row();
        Self::move_down(stdout, move_down)?;
        cursor = Cursor::new(next.last_row());
      }

      for _ in next.len()..presented.frame.len() {
        write!(stdout, "\r\n")?;
        queue!(stdout, Clear(ClearType::CurrentLine))?;
      }

      Self::move_up(stdout, diff.deleted_tail_len())?;
    }

    write!(stdout, "\x1b[?2026l")?;

    Ok(PresentedFrame::new(cursor, next, next_viewport))
  }

  fn present_noop(&self, next: &Frame) -> PresentedFrame {
    let presented = self.presented.as_ref().unwrap();

    PresentedFrame::new(
      presented.cursor,
      next.clone(),
      self.previous_viewport(next),
    )
  }

  fn previous_viewport(&self, next: &Frame) -> Viewport {
    let presented = self.presented.as_ref().unwrap();

    let height = next.dimensions.height();

    if presented.frame.dimensions.height == next.dimensions.height {
      Viewport::new(presented.viewport.top(), height)
    } else {
      Viewport::anchored_to_bottom(
        presented
          .viewport
          .top()
          .saturating_add(presented.frame.dimensions.height()),
        height,
      )
    }
  }

  fn render_op(&self, next: &Frame) -> RenderOp {
    let Some(presented) = &self.presented else {
      return RenderOp::Full { clear: false };
    };

    if presented.frame.dimensions.width != next.dimensions.width {
      return RenderOp::Full { clear: true };
    }

    if presented.frame.dimensions.height != next.dimensions.height
      && !Self::is_termux_session()
    {
      return RenderOp::Full { clear: true };
    }

    if next.len() < self.max_lines_rendered
      && env::var_os("KOTOMORI_CLEAR_ON_SHRINK").is_some()
    {
      return RenderOp::Full { clear: true };
    }

    let Some(diff) = Diff::between(&presented.frame, next) else {
      return RenderOp::Noop;
    };

    let previous_viewport = self.previous_viewport(next);

    if diff.changed.first < previous_viewport.top() {
      return RenderOp::Full { clear: true };
    }

    if diff.is_pure_tail_delete()
      && next.last_row() < previous_viewport.top()
      && !next.is_empty()
    {
      return RenderOp::Full { clear: true };
    }

    if diff.deleted_tail_len() > next.dimensions.height() {
      return RenderOp::Full { clear: true };
    }

    RenderOp::Patch { diff }
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

  fn frame(lines: &[&str], width: u16, height: u16) -> Frame {
    Frame::new(
      lines.iter().map(|line| (*line).into()).collect(),
      Dimensions { height, width },
    )
  }

  fn renderer(lines: &[&str], width: u16, height: u16) -> Renderer {
    let frame = frame(lines, width, height);

    Renderer {
      max_lines_rendered: frame.len(),
      presented: Some(PresentedFrame::new(
        Cursor::new(frame.last_row()),
        frame,
        Viewport::anchored_to_bottom(lines.len(), usize::from(height)),
      )),
    }
  }

  #[test]
  fn appending_lines_scrolls() {
    let mut subject = renderer(&["foo"], 80, 24);

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
  fn appending_past_viewport_scrolls_instead_of_redrawing() {
    let mut subject = renderer(&["foo", "bar"], 80, 1);

    let mut stdout = Vec::new();

    subject
      .draw_rendered(
        &mut stdout,
        vec!["foo".into(), "bar".into(), "baz".into()],
        80,
        1,
      )
      .unwrap();

    assert_eq!(
      String::from_utf8(stdout).unwrap(),
      "\x1b[?2026h\r\n\x1b[2Kbaz\x1b[?2026l",
    );
  }

  #[test]
  fn finishing_moves_cursor_to_last_rendered_line() {
    let mut subject = renderer(&["foo", "bar", "baz"], 80, 24);
    subject.presented.as_mut().unwrap().cursor = Cursor::new(1);

    let mut stdout = Vec::new();

    subject.finish(&mut stdout).unwrap();

    assert_eq!(String::from_utf8(stdout).unwrap(), "\x1b[1B");
    assert_eq!(subject.presented.unwrap().cursor, Cursor::new(2));
  }

  #[test]
  fn full_render_can_clear_screen_and_scrollback() {
    let mut stdout = Vec::new();

    Renderer::full_render(&mut stdout, &frame(&["bar", "baz"], 80, 10), true)
      .unwrap();

    assert_eq!(
      String::from_utf8(stdout).unwrap(),
      "\x1b[?2026h\x1b[1;1H\x1b[2J\x1b[3Jbar\r\nbaz\x1b[?2026l",
    );
  }

  #[test]
  fn redraws_only_changed_line() {
    let mut subject = renderer(&["foo", "bar", "baz"], 80, 24);

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
    let mut subject = renderer(&["foo", "bar", "baz"], 80, 2);

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
    let mut subject = renderer(&["foo"], 80, 24);

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
    let mut subject = renderer(&["foo"], 80, 24);

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
    let mut subject = renderer(&["foo", "bar", "baz"], 80, 24);

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
