use super::*;

#[derive(Debug)]
pub(crate) struct Renderer {
  presented: Option<PresentedFrame>,
}

impl Renderer {
  fn clear_deleted_tail(
    stdout: &mut impl Write,
    presented: &PresentedFrame,
    next: &Frame,
    diff: Diff,
    viewport: Viewport,
  ) -> Result<PresentedFrame> {
    let target_row = next.last_row();

    stdout.begin_synchronized_update()?;

    stdout.move_by(presented.cursor.diff_to(viewport, target_row, viewport))?;

    let deleted_tail_len = diff.deleted_tail_len();

    if !next.is_empty() {
      write!(stdout, "\r")?;

      if deleted_tail_len > 0 {
        stdout.move_down(1)?;
      }
    }

    for index in diff.changed.first..=diff.changed.last {
      if index > diff.changed.first {
        stdout.move_down(1)?;
      }

      write!(stdout, "\r")?;

      stdout.clear_line()?;
    }

    stdout
      .move_up(deleted_tail_len.saturating_sub(usize::from(next.is_empty())))?;

    stdout.end_synchronized_update()?;

    Ok(PresentedFrame::new(
      Cursor::new(target_row),
      next.clone(),
      viewport,
    ))
  }

  pub(crate) fn draw(
    &mut self,
    stdout: &mut impl Write,
    component: &impl Component,
  ) -> Result {
    self.draw_frame(stdout, Self::frame(component)?)?;

    stdout.flush()?;

    Ok(())
  }

  fn draw_frame(&mut self, stdout: &mut impl Write, next: Frame) -> Result {
    let plan = RenderPlan::between(self.presented.as_ref(), &next);

    let presented = match plan {
      RenderPlan::Full { clear } => Self::full_render(stdout, &next, clear)?,
      RenderPlan::NoOperation => return Ok(()),
      RenderPlan::Patch { diff } => Self::patch_render(
        stdout,
        self.presented.as_ref().unwrap(),
        next,
        self.presented.as_ref().unwrap().viewport,
        diff,
      )?,
    };

    self.presented = Some(presented);

    Ok(())
  }

  pub(crate) fn finish(&mut self, stdout: &mut impl Write) -> Result {
    let Some(presented) = &self.presented else {
      return Ok(());
    };

    stdout.move_by(presented.cursor.diff_to(
      presented.viewport,
      presented.frame.last_row(),
      presented.viewport,
    ))?;

    if let Some(presented) = &mut self.presented {
      presented.cursor = Cursor::new(presented.frame.last_row());
    }

    Ok(())
  }

  fn frame(component: &impl Component) -> Result<Frame> {
    let (width, height) =
      crossterm_terminal::size().context("failed to read terminal size")?;

    let lines = component
      .render(width)
      .into_iter()
      .flat_map(|line| line.render(width))
      .map(|line| format!("{line}{}", Style::None.sequence()))
      .collect::<Vec<_>>();

    Ok(Frame::new(
      lines,
      Dimensions {
        height: usize::from(height),
        width,
      },
    ))
  }

  fn full_render(
    stdout: &mut impl Write,
    next: &Frame,
    clear: bool,
  ) -> Result<PresentedFrame> {
    stdout.begin_synchronized_update()?;

    if clear {
      stdout.clear_screen()?;
    }

    stdout.write_lines(&next.lines)?;

    stdout.end_synchronized_update()?;

    Ok(next.clone().into())
  }

  fn line_feed(
    stdout: &mut impl Write,
    viewport: &mut Viewport,
    row: usize,
  ) -> Result {
    write!(stdout, "\r\n")?;

    if viewport.screen_row(row) >= viewport.height.saturating_sub(1) {
      *viewport = viewport.scrolled_down(1);
    }

    Ok(())
  }

  pub(crate) fn new() -> Self {
    Self { presented: None }
  }

  fn patch_render(
    stdout: &mut impl Write,
    presented: &PresentedFrame,
    next: Frame,
    mut viewport: Viewport,
    diff: Diff,
  ) -> Result<PresentedFrame> {
    if diff.is_pure_tail_delete() {
      return Self::clear_deleted_tail(
        stdout, presented, &next, diff, viewport,
      );
    }

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

    stdout.begin_synchronized_update()?;

    if move_target_row > viewport.bottom() {
      let move_to_bottom = viewport
        .height
        .saturating_sub(1)
        .saturating_sub(viewport.screen_row(cursor.row));

      stdout.move_down(move_to_bottom)?;

      let bottom = viewport.bottom();

      for row in bottom..move_target_row {
        Self::line_feed(stdout, &mut viewport, row)?;
      }

      cursor = Cursor::new(move_target_row);
    }

    stdout.move_by(cursor.diff_to(viewport, move_target_row, viewport))?;

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

      stdout.write_line(line)?;
    }

    let mut cursor = Cursor::new(*writable_range.end());

    if diff.deleted_tail_len() > 0 {
      if cursor.row < next.last_row() {
        let move_down = next.last_row() - cursor.row;
        stdout.move_down(move_down)?;
        cursor = Cursor::new(next.last_row());
      }

      for _ in next.len()..presented.frame.len() {
        Self::line_feed(stdout, &mut viewport, cursor.row)?;
        cursor = Cursor::new(cursor.row.saturating_add(1));
        stdout.clear_line()?;
      }

      stdout.move_up(diff.deleted_tail_len())?;
      cursor = Cursor::new(next.last_row());
    }

    stdout.end_synchronized_update()?;

    Ok(PresentedFrame::new(cursor, next, viewport))
  }
}

#[cfg(test)]
mod tests {
  use super::*;

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
      presented: Some(PresentedFrame::new(
        Cursor::new(frame.last_row()),
        frame,
        Viewport::anchored_to_bottom(1, 1),
      )),
    };

    let mut stdout = Vec::new();

    subject
      .draw_frame(
        &mut stdout,
        Frame::new(
          vec!["foo".into(), String::new()],
          Dimensions {
            height: 1,
            width: 80,
          },
        ),
      )
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
  fn appending_lines_scrolls() {
    let frame = Frame::new(
      vec!["foo".into()],
      Dimensions {
        height: 24,
        width: 80,
      },
    );

    let mut subject = Renderer {
      presented: Some(PresentedFrame::new(
        Cursor::new(frame.last_row()),
        frame,
        Viewport::anchored_to_bottom(1, 24),
      )),
    };

    let mut stdout = Vec::new();

    subject
      .draw_frame(
        &mut stdout,
        Frame::new(
          vec!["foo".into(), "bar".into()],
          Dimensions {
            height: 24,
            width: 80,
          },
        ),
      )
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
      presented: Some(PresentedFrame::new(
        Cursor::new(frame.last_row()),
        frame,
        Viewport::anchored_to_bottom(2, 1),
      )),
    };

    let mut stdout = Vec::new();

    subject
      .draw_frame(
        &mut stdout,
        Frame::new(
          vec!["foo".into(), "bar".into(), "baz".into()],
          Dimensions {
            height: 1,
            width: 80,
          },
        ),
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
  fn clears_visible_lines_when_frame_becomes_empty() {
    let frame = Frame::new(
      vec!["foo".into()],
      Dimensions {
        height: 24,
        width: 80,
      },
    );

    let mut subject = Renderer {
      presented: Some(PresentedFrame::new(
        Cursor::new(frame.last_row()),
        frame,
        Viewport::anchored_to_bottom(1, 24),
      )),
    };

    let mut stdout = Vec::new();

    subject
      .draw_frame(
        &mut stdout,
        Frame::new(
          Vec::new(),
          Dimensions {
            height: 24,
            width: 80,
          },
        ),
      )
      .unwrap();

    assert_eq!(
      String::from_utf8(stdout).unwrap(),
      "\x1b[?2026h\r\x1b[2K\x1b[?2026l",
    );

    assert_eq!(
      subject.presented.as_ref().unwrap().frame.lines,
      Vec::<String>::new(),
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
      "\x1b[?2026h\x1b[2J\x1b[1;1H\x1b[3J\x1b[1G\x1b[2Kbar\r\n\x1b[1G\x1b[2Kbaz\x1b[?2026l",
    );
  }

  #[test]
  fn full_render_rebuilds_scrollback_when_clearing() {
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
      "\x1b[?2026h\x1b[2J\x1b[1;1H\x1b[3J\x1b[1G\x1b[2Kfoo\r\n\x1b[1G\x1b[2Kbar\r\n\x1b[1G\x1b[2Kbaz\x1b[?2026l",
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
  fn patches_when_shrink_moves_viewport_up() {
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
      presented: Some(PresentedFrame::new(
        Cursor::new(frame.last_row()),
        frame,
        Viewport::anchored_to_bottom(5, 3),
      )),
    };

    let mut stdout = Vec::new();

    subject
      .draw_frame(
        &mut stdout,
        Frame::new(
          vec!["foo".into(), "bar".into(), "bob".into(), "qux".into()],
          Dimensions {
            height: 3,
            width: 80,
          },
        ),
      )
      .unwrap();

    assert_eq!(
      String::from_utf8(stdout).unwrap(),
      "\x1b[?2026h\x1b[2A\r\x1b[2Kbob\r\n\x1b[2Kqux\r\n\x1b[2K\x1b[1A\x1b[?2026l",
    );

    assert_eq!(
      subject.presented.as_ref().unwrap().viewport,
      Viewport::new(2, 3),
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
      presented: Some(PresentedFrame::new(
        Cursor::new(frame.last_row()),
        frame,
        Viewport::anchored_to_bottom(3, 24),
      )),
    };

    let mut stdout = Vec::new();

    subject
      .draw_frame(
        &mut stdout,
        Frame::new(
          vec!["foo".into(), "qux".into(), "baz".into()],
          Dimensions {
            height: 24,
            width: 80,
          },
        ),
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
      presented: Some(PresentedFrame::new(
        Cursor::new(frame.last_row()),
        frame,
        Viewport::anchored_to_bottom(3, 2),
      )),
    };

    let mut stdout = Vec::new();

    subject
      .draw_frame(
        &mut stdout,
        Frame::new(
          vec!["qux".into(), "bar".into(), "baz".into()],
          Dimensions {
            height: 2,
            width: 80,
          },
        ),
      )
      .unwrap();

    assert_eq!(
      String::from_utf8(stdout).unwrap(),
      "\x1b[?2026h\x1b[2J\x1b[1;1H\x1b[3J\x1b[1G\x1b[2Kqux\r\n\x1b[1G\x1b[2Kbar\r\n\x1b[1G\x1b[2Kbaz\x1b[?2026l",
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
      presented: Some(PresentedFrame::new(
        Cursor::new(frame.last_row()),
        frame,
        Viewport::anchored_to_bottom(1, 24),
      )),
    };

    let mut stdout = Vec::new();

    subject
      .draw_frame(
        &mut stdout,
        Frame::new(
          vec!["foo".into(), "bar".into()],
          Dimensions {
            height: 25,
            width: 80,
          },
        ),
      )
      .unwrap();

    assert_eq!(
      String::from_utf8(stdout).unwrap(),
      "\x1b[?2026h\x1b[2J\x1b[1;1H\x1b[3J\x1b[1G\x1b[2Kfoo\r\n\x1b[1G\x1b[2Kbar\x1b[?2026l",
    );
  }

  #[test]
  fn redraws_screen_when_height_changes_without_line_changes() {
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
      presented: Some(PresentedFrame::new(
        Cursor::new(frame.last_row()),
        frame.clone(),
        Viewport::anchored_to_bottom(5, 3),
      )),
    };

    let mut stdout = Vec::new();

    subject
      .draw_frame(
        &mut stdout,
        Frame::new(
          frame.lines,
          Dimensions {
            height: 4,
            width: 80,
          },
        ),
      )
      .unwrap();

    assert_eq!(
      String::from_utf8(stdout).unwrap(),
      "\x1b[?2026h\x1b[2J\x1b[1;1H\x1b[3J\x1b[1G\x1b[2Kfoo\r\n\x1b[1G\x1b[2Kbar\r\n\x1b[1G\x1b[2Kbaz\r\n\x1b[1G\x1b[2Kqux\r\n\x1b[1G\x1b[2Kquux\x1b[?2026l",
    );

    assert_eq!(
      subject.presented.as_ref().unwrap().viewport,
      Viewport::new(1, 4),
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
      presented: Some(PresentedFrame::new(
        Cursor::new(frame.last_row()),
        frame,
        Viewport::anchored_to_bottom(1, 24),
      )),
    };

    let mut stdout = Vec::new();

    subject
      .draw_frame(
        &mut stdout,
        Frame::new(
          vec!["foo".into(), "bar".into()],
          Dimensions {
            height: 24,
            width: 81,
          },
        ),
      )
      .unwrap();

    assert_eq!(
      String::from_utf8(stdout).unwrap(),
      "\x1b[?2026h\x1b[2J\x1b[1;1H\x1b[3J\x1b[1G\x1b[2Kfoo\r\n\x1b[1G\x1b[2Kbar\x1b[?2026l",
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
      presented: Some(PresentedFrame::new(
        Cursor::new(frame.last_row()),
        frame,
        Viewport::anchored_to_bottom(3, 24),
      )),
    };

    let mut stdout = Vec::new();

    subject
      .draw_frame(
        &mut stdout,
        Frame::new(
          vec!["foo".into()],
          Dimensions {
            height: 24,
            width: 80,
          },
        ),
      )
      .unwrap();

    assert_eq!(
      String::from_utf8(stdout).unwrap(),
      "\x1b[?2026h\x1b[2A\r\x1b[1B\r\x1b[2K\x1b[1B\r\x1b[2K\x1b[2A\x1b[?2026l",
    );
  }
}
