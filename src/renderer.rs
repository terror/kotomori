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

    let operation = self.render_operation(&next);

    let presented = match operation {
      RenderOperation::Full { clear } => {
        Self::full_render(stdout, &next, clear)?
      }
      RenderOperation::NoOperation => self.present_no_operation(&next),
      RenderOperation::Patch { diff } => {
        self.patch_render(stdout, next, diff)?
      }
    };

    self.max_lines_rendered =
      if matches!(operation, RenderOperation::Full { clear: true }) {
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
      queue!(stdout, MoveTo(0, 0), Clear(ClearType::All))?;
    }

    let lines = if clear {
      &next.lines[next.len().saturating_sub(next.dimensions.height())..]
    } else {
      &next.lines
    };

    Self::write_lines(stdout, lines)?;

    write!(stdout, "\x1b[?2026l")?;

    Ok(next.clone().into())
  }

  fn is_termux_session() -> bool {
    env::var_os("TERMUX_VERSION").is_some()
  }

  fn line_feed(
    stdout: &mut impl Write,
    viewport: &mut Viewport,
    row: usize,
  ) -> Result {
    write!(stdout, "\r\n")?;

    if viewport.screen_row(row) >= viewport.height().saturating_sub(1) {
      *viewport = viewport.scrolled_down(1);
    }

    Ok(())
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
    let mut viewport = previous_viewport;

    write!(stdout, "\x1b[?2026h")?;

    if move_target_row > viewport.bottom() {
      let move_to_bottom = viewport
        .height()
        .saturating_sub(1)
        .saturating_sub(viewport.screen_row(cursor.row()));

      Self::move_down(stdout, move_to_bottom)?;

      let bottom = viewport.bottom();

      for row in bottom..move_target_row {
        Self::line_feed(stdout, &mut viewport, row)?;
      }

      cursor = Cursor::new(move_target_row);
    }

    Self::move_by(stdout, cursor.diff_to(viewport, move_target_row, viewport))?;

    if append_start {
      Self::line_feed(stdout, &mut viewport, move_target_row)?;
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
        Self::line_feed(stdout, &mut viewport, index.saturating_sub(1))?;
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
        Self::line_feed(stdout, &mut viewport, cursor.row())?;
        cursor = Cursor::new(cursor.row().saturating_add(1));
        queue!(stdout, Clear(ClearType::CurrentLine))?;
      }

      Self::move_up(stdout, diff.deleted_tail_len())?;
      cursor = Cursor::new(next.last_row());
    }

    write!(stdout, "\x1b[?2026l")?;

    Ok(PresentedFrame::new(cursor, next, viewport))
  }

  fn present_no_operation(&self, next: &Frame) -> PresentedFrame {
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

  fn render_operation(&self, next: &Frame) -> RenderOperation {
    let Some(presented) = &self.presented else {
      return RenderOperation::Full { clear: false };
    };

    if presented.frame.dimensions.width != next.dimensions.width {
      return RenderOperation::Full { clear: true };
    }

    if presented.frame.dimensions.height != next.dimensions.height
      && !Self::is_termux_session()
    {
      return RenderOperation::Full { clear: true };
    }

    if next.len() < self.max_lines_rendered
      && env::var_os("KOTOMORI_CLEAR_ON_SHRINK").is_some()
    {
      return RenderOperation::Full { clear: true };
    }

    let Some(diff) = Diff::between(&presented.frame, next) else {
      return RenderOperation::NoOperation;
    };

    let previous_viewport = self.previous_viewport(next);

    let next_viewport =
      Viewport::anchored_to_bottom(next.len(), next.dimensions.height());

    if next.is_empty() || next_viewport.top() < previous_viewport.top() {
      return RenderOperation::Full { clear: true };
    }

    if diff.changed.first < previous_viewport.top() {
      return RenderOperation::Full { clear: true };
    }

    if diff.is_pure_tail_delete()
      && next.last_row() < previous_viewport.top()
      && !next.is_empty()
    {
      return RenderOperation::Full { clear: true };
    }

    if diff.deleted_tail_len() > next.dimensions.height() {
      return RenderOperation::Full { clear: true };
    }

    RenderOperation::Patch { diff }
  }

  fn write_lines(stdout: &mut impl Write, lines: &[String]) -> Result {
    for (index, line) in lines.iter().enumerate() {
      if index > 0 {
        write!(stdout, "\r\n")?;
      }

      queue!(stdout, MoveToColumn(0), Clear(ClearType::CurrentLine))?;

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
    let frame = Frame::new(
      vec!["foo".into()],
      Dimensions {
        height: 24,
        width: 80,
      },
    );

    let mut subject = Renderer {
      max_lines_rendered: frame.len(),
      presented: Some(PresentedFrame::new(
        Cursor::new(frame.last_row()),
        frame,
        Viewport::anchored_to_bottom(1, 24),
      )),
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
  fn appending_past_viewport_scrolls_instead_of_redrawing() {
    let frame = Frame::new(
      vec!["foo".into(), "bar".into()],
      Dimensions {
        height: 1,
        width: 80,
      },
    );

    let mut subject = Renderer {
      max_lines_rendered: frame.len(),
      presented: Some(PresentedFrame::new(
        Cursor::new(frame.last_row()),
        frame,
        Viewport::anchored_to_bottom(2, 1),
      )),
    };

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

    assert_eq!(
      subject.presented.as_ref().unwrap().viewport,
      Viewport::anchored_to_bottom(3, 1),
    );
    assert_eq!(subject.presented.as_ref().unwrap().cursor, Cursor::new(2),);
  }

  #[test]
  fn appending_blank_line_scrolls() {
    let frame = Frame::new(
      vec!["foo".into()],
      Dimensions {
        height: 1,
        width: 80,
      },
    );

    let mut subject = Renderer {
      max_lines_rendered: frame.len(),
      presented: Some(PresentedFrame::new(
        Cursor::new(frame.last_row()),
        frame,
        Viewport::anchored_to_bottom(1, 1),
      )),
    };

    let mut stdout = Vec::new();

    subject
      .draw_rendered(&mut stdout, vec!["foo".into(), String::new()], 80, 1)
      .unwrap();

    assert_eq!(
      String::from_utf8(stdout).unwrap(),
      "\x1b[?2026h\r\n\x1b[2K\x1b[?2026l",
    );

    assert_eq!(
      subject.presented.as_ref().unwrap().viewport,
      Viewport::anchored_to_bottom(2, 1),
    );
  }

  #[test]
  fn finishing_moves_cursor_to_last_rendered_line() {
    let frame = Frame::new(
      vec!["foo".into(), "bar".into(), "baz".into()],
      Dimensions {
        height: 24,
        width: 80,
      },
    );

    let mut subject = Renderer {
      max_lines_rendered: frame.len(),
      presented: Some(PresentedFrame::new(
        Cursor::new(frame.last_row()),
        frame,
        Viewport::anchored_to_bottom(3, 24),
      )),
    };

    subject.presented.as_mut().unwrap().cursor = Cursor::new(1);

    let mut stdout = Vec::new();

    subject.finish(&mut stdout).unwrap();

    assert_eq!(String::from_utf8(stdout).unwrap(), "\x1b[1B");
    assert_eq!(subject.presented.unwrap().cursor, Cursor::new(2));
  }

  #[test]
  fn full_render_clears_screen() {
    let mut stdout = Vec::new();

    Renderer::full_render(
      &mut stdout,
      &Frame::new(
        vec!["bar".into(), "baz".into()],
        Dimensions {
          height: 10,
          width: 80,
        },
      ),
      true,
    )
    .unwrap();

    assert_eq!(
      String::from_utf8(stdout).unwrap(),
      "\x1b[?2026h\x1b[1;1H\x1b[2J\x1b[1G\x1b[2Kbar\r\n\x1b[1G\x1b[2Kbaz\x1b[?2026l",
    );
  }

  #[test]
  fn full_render_clips_to_height_when_clearing() {
    let mut stdout = Vec::new();

    Renderer::full_render(
      &mut stdout,
      &Frame::new(
        vec!["foo".into(), "bar".into(), "baz".into()],
        Dimensions {
          height: 2,
          width: 80,
        },
      ),
      true,
    )
    .unwrap();

    assert_eq!(
      String::from_utf8(stdout).unwrap(),
      "\x1b[?2026h\x1b[1;1H\x1b[2J\x1b[1G\x1b[2Kbar\r\n\x1b[1G\x1b[2Kbaz\x1b[?2026l",
    );
  }

  #[test]
  fn full_render_without_clear_prepares_lines() {
    let mut stdout = Vec::new();

    Renderer::full_render(
      &mut stdout,
      &Frame::new(
        vec!["foo".into(), "bar".into()],
        Dimensions {
          height: 10,
          width: 80,
        },
      ),
      false,
    )
    .unwrap();

    assert_eq!(
      String::from_utf8(stdout).unwrap(),
      "\x1b[?2026h\x1b[1G\x1b[2Kfoo\r\n\x1b[1G\x1b[2Kbar\x1b[?2026l",
    );
  }

  #[test]
  fn redraws_only_changed_line() {
    let frame = Frame::new(
      vec!["foo".into(), "bar".into(), "baz".into()],
      Dimensions {
        height: 24,
        width: 80,
      },
    );

    let mut subject = Renderer {
      max_lines_rendered: frame.len(),
      presented: Some(PresentedFrame::new(
        Cursor::new(frame.last_row()),
        frame,
        Viewport::anchored_to_bottom(3, 24),
      )),
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
    let frame = Frame::new(
      vec!["foo".into(), "bar".into(), "baz".into()],
      Dimensions {
        height: 2,
        width: 80,
      },
    );

    let mut subject = Renderer {
      max_lines_rendered: frame.len(),
      presented: Some(PresentedFrame::new(
        Cursor::new(frame.last_row()),
        frame,
        Viewport::anchored_to_bottom(3, 2),
      )),
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
      "\x1b[?2026h\x1b[1;1H\x1b[2J\x1b[1G\x1b[2Kbar\r\n\x1b[1G\x1b[2Kbaz\x1b[?2026l",
    );
  }

  #[test]
  fn redraws_screen_when_height_changes() {
    let frame = Frame::new(
      vec!["foo".into()],
      Dimensions {
        height: 24,
        width: 80,
      },
    );

    let mut subject = Renderer {
      max_lines_rendered: frame.len(),
      presented: Some(PresentedFrame::new(
        Cursor::new(frame.last_row()),
        frame,
        Viewport::anchored_to_bottom(1, 24),
      )),
    };

    let mut stdout = Vec::new();

    subject
      .draw_rendered(&mut stdout, vec!["foo".into(), "bar".into()], 80, 25)
      .unwrap();

    assert_eq!(
      String::from_utf8(stdout).unwrap(),
      "\x1b[?2026h\x1b[1;1H\x1b[2J\x1b[1G\x1b[2Kfoo\r\n\x1b[1G\x1b[2Kbar\x1b[?2026l",
    );
  }

  #[test]
  fn redraws_screen_when_width_changes() {
    let frame = Frame::new(
      vec!["foo".into()],
      Dimensions {
        height: 24,
        width: 80,
      },
    );

    let mut subject = Renderer {
      max_lines_rendered: frame.len(),
      presented: Some(PresentedFrame::new(
        Cursor::new(frame.last_row()),
        frame,
        Viewport::anchored_to_bottom(1, 24),
      )),
    };

    let mut stdout = Vec::new();

    subject
      .draw_rendered(&mut stdout, vec!["foo".into(), "bar".into()], 81, 24)
      .unwrap();

    assert_eq!(
      String::from_utf8(stdout).unwrap(),
      "\x1b[?2026h\x1b[1;1H\x1b[2J\x1b[1G\x1b[2Kfoo\r\n\x1b[1G\x1b[2Kbar\x1b[?2026l",
    );
  }

  #[test]
  fn redraws_screen_when_shrink_moves_viewport_up() {
    let frame = Frame::new(
      vec![
        "foo".into(),
        "bar".into(),
        "baz".into(),
        "qux".into(),
        "quux".into(),
      ],
      Dimensions {
        height: 3,
        width: 80,
      },
    );

    let mut subject = Renderer {
      max_lines_rendered: frame.len(),
      presented: Some(PresentedFrame::new(
        Cursor::new(frame.last_row()),
        frame,
        Viewport::anchored_to_bottom(5, 3),
      )),
    };

    let mut stdout = Vec::new();

    subject
      .draw_rendered(
        &mut stdout,
        vec!["foo".into(), "bar".into(), "bob".into(), "qux".into()],
        80,
        3,
      )
      .unwrap();

    assert_eq!(
      String::from_utf8(stdout).unwrap(),
      "\x1b[?2026h\x1b[1;1H\x1b[2J\x1b[1G\x1b[2Kbar\r\n\x1b[1G\x1b[2Kbob\r\n\x1b[1G\x1b[2Kqux\x1b[?2026l",
    );

    assert_eq!(
      subject.presented.as_ref().unwrap().viewport,
      Viewport::anchored_to_bottom(4, 3),
    );
  }

  #[test]
  fn redraws_screen_when_frame_becomes_empty() {
    let frame = Frame::new(
      vec!["foo".into()],
      Dimensions {
        height: 24,
        width: 80,
      },
    );

    let mut subject = Renderer {
      max_lines_rendered: frame.len(),
      presented: Some(PresentedFrame::new(
        Cursor::new(frame.last_row()),
        frame,
        Viewport::anchored_to_bottom(1, 24),
      )),
    };

    let mut stdout = Vec::new();

    subject
      .draw_rendered(&mut stdout, Vec::new(), 80, 24)
      .unwrap();

    assert_eq!(
      String::from_utf8(stdout).unwrap(),
      "\x1b[?2026h\x1b[1;1H\x1b[2J\x1b[?2026l",
    );

    assert_eq!(
      subject.presented.as_ref().unwrap().frame.lines,
      Vec::<String>::new(),
    );
  }

  #[test]
  fn removes_deleted_tail_lines() {
    let frame = Frame::new(
      vec!["foo".into(), "bar".into(), "baz".into()],
      Dimensions {
        height: 24,
        width: 80,
      },
    );

    let mut subject = Renderer {
      max_lines_rendered: frame.len(),
      presented: Some(PresentedFrame::new(
        Cursor::new(frame.last_row()),
        frame,
        Viewport::anchored_to_bottom(3, 24),
      )),
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

    assert_eq!(
      String::from_utf8(stdout).unwrap(),
      "\x1b[1G\x1b[2Kfoo\r\n\x1b[1G\x1b[2Kbar",
    );
  }
}
