use super::*;

#[derive(Debug)]
pub(crate) struct Renderer<W: Write = BufWriter<Stdout>> {
  presented: Option<PresentedFrame>,
  stdout: W,
}

impl Renderer {
  pub(crate) fn new() -> Result<Self> {
    enable_raw_mode().context("failed to enable raw mode")?;

    let mut stdout = BufWriter::new(io::stdout());

    queue!(stdout, Hide).context("failed to hide cursor")?;

    Ok(Self {
      presented: None,
      stdout,
    })
  }
}

impl<W: Write> Renderer<W> {
  pub(crate) fn draw(&mut self, component: &impl Component) -> Result {
    self.draw_frame(Self::frame(component)?)?;

    self.stdout.flush()?;

    Ok(())
  }

  fn draw_frame(&mut self, next: Frame) -> Result {
    let plan = RenderPlan::between(self.presented.as_ref(), &next);

    self.presented = Some(match plan {
      RenderPlan::Full { clear } => self.full_render(next, clear)?,
      RenderPlan::NoOperation => return Ok(()),
      RenderPlan::Patch { diff } => self.patch_render(next, diff)?,
    });

    Ok(())
  }

  pub(crate) fn finish(&mut self) -> Result {
    let Some(presented) = &self.presented else {
      return Ok(());
    };

    let target_row = presented.frame.last_row();

    self
      .stdout
      .move_up(presented.cursor_row.saturating_sub(target_row))?;

    self
      .stdout
      .move_down(target_row.saturating_sub(presented.cursor_row))?;

    if let Some(presented) = &mut self.presented {
      presented.cursor_row = presented.frame.last_row();
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
    &mut self,
    next: Frame,
    clear: bool,
  ) -> Result<PresentedFrame> {
    self.stdout.begin_synchronized_update()?;

    if clear {
      self.stdout.clear_screen()?;
    }

    self.stdout.write_lines(&next.lines)?;

    self.stdout.end_synchronized_update()?;

    Ok(next.into())
  }

  fn line_feed(
    &mut self,
    cursor_row: &mut usize,
    viewport: &mut Viewport,
  ) -> Result {
    write!(self.stdout, "\r\n")?;

    if viewport.screen_row(*cursor_row) >= viewport.height.saturating_sub(1) {
      *viewport = viewport.scrolled_down(1);
    }

    *cursor_row = cursor_row.saturating_add(1);

    Ok(())
  }

  fn patch_render(
    &mut self,
    next: Frame,
    diff: Diff,
  ) -> Result<PresentedFrame> {
    let (mut viewport, presented_len, mut cursor_row) = {
      let presented = self.presented.as_ref().unwrap();

      (
        presented.viewport,
        presented.frame.len(),
        presented.cursor_row,
      )
    };

    let append_start =
      diff.changed.first > 0 && diff.changed.first == presented_len;

    let move_target_row = if append_start {
      diff.changed.first.saturating_sub(1)
    } else {
      diff.changed.first
    };

    self.stdout.begin_synchronized_update()?;

    if move_target_row > viewport.bottom() {
      let move_to_bottom = viewport
        .height
        .saturating_sub(1)
        .saturating_sub(viewport.screen_row(cursor_row));

      self.stdout.move_down(move_to_bottom)?;

      let bottom = viewport.bottom();

      cursor_row = bottom;

      for _ in bottom..move_target_row {
        self.line_feed(&mut cursor_row, &mut viewport)?;
      }
    }

    self
      .stdout
      .move_up(cursor_row.saturating_sub(move_target_row))?;

    self
      .stdout
      .move_down(move_target_row.saturating_sub(cursor_row))?;

    cursor_row = move_target_row;

    if append_start {
      self.line_feed(&mut cursor_row, &mut viewport)?;
    } else {
      write!(self.stdout, "\r")?;
    }

    for row in diff.changed.first..=diff.changed.last {
      if row > diff.changed.first {
        self.line_feed(&mut cursor_row, &mut viewport)?;
      }

      if let Some(line) = next.lines.get(row) {
        self.stdout.write_line(line)?;
      } else {
        self.stdout.clear_line()?;
      }
    }

    let last_row = next.last_row();

    self.stdout.move_up(cursor_row.saturating_sub(last_row))?;
    self.stdout.move_down(last_row.saturating_sub(cursor_row))?;

    cursor_row = last_row;

    self.stdout.end_synchronized_update()?;

    Ok(PresentedFrame::new(cursor_row, next, viewport))
  }
}

impl<W: Default + Write> Default for Renderer<W> {
  fn default() -> Self {
    Self {
      presented: None,
      stdout: W::default(),
    }
  }
}

#[cfg(not(test))]
impl<W: Write> Drop for Renderer<W> {
  fn drop(&mut self) {
    let _ = self.stdout.end_synchronized_update();

    let _ = crossterm::execute!(
      self.stdout,
      crossterm::cursor::MoveToColumn(0),
      crossterm::cursor::MoveToNextLine(1),
      crossterm::cursor::Show,
    );

    let _ = self.stdout.flush();

    if let Err(error) = crossterm::terminal::disable_raw_mode() {
      eprintln!("failed to restore terminal: {error}");
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  type TestRenderer = Renderer<Vec<u8>>;

  #[test]
  fn appending_blank_line_scrolls() {
    let frame = Frame::new(
      vec!["foo".into()],
      Dimensions {
        height: 1,
        width: 80,
      },
    );

    let mut renderer = TestRenderer {
      presented: Some(PresentedFrame::new(
        frame.last_row(),
        frame,
        Viewport::anchored_to_bottom(1, 1),
      )),
      ..Default::default()
    };

    renderer
      .draw_frame(Frame::new(
        vec!["foo".into(), String::new()],
        Dimensions {
          height: 1,
          width: 80,
        },
      ))
      .unwrap();

    assert_eq!(
      String::from_utf8(renderer.stdout.clone()).unwrap(),
      "\x1b[?2026h\r\n\x1b[2K\x1b[?2026l",
    );

    assert_eq!(
      renderer.presented.as_ref().unwrap().viewport,
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

    let mut renderer = TestRenderer {
      presented: Some(PresentedFrame::new(
        frame.last_row(),
        frame,
        Viewport::anchored_to_bottom(1, 24),
      )),
      ..Default::default()
    };

    renderer
      .draw_frame(Frame::new(
        vec!["foo".into(), "bar".into()],
        Dimensions {
          height: 24,
          width: 80,
        },
      ))
      .unwrap();

    assert_eq!(
      String::from_utf8(renderer.stdout.clone()).unwrap(),
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

    let mut renderer = TestRenderer {
      presented: Some(PresentedFrame::new(
        frame.last_row(),
        frame,
        Viewport::anchored_to_bottom(2, 1),
      )),
      ..Default::default()
    };

    renderer
      .draw_frame(Frame::new(
        vec!["foo".into(), "bar".into(), "baz".into()],
        Dimensions {
          height: 1,
          width: 80,
        },
      ))
      .unwrap();

    assert_eq!(
      String::from_utf8(renderer.stdout.clone()).unwrap(),
      "\x1b[?2026h\r\n\x1b[2Kbaz\x1b[?2026l",
    );

    assert_eq!(
      renderer.presented.as_ref().unwrap().viewport,
      Viewport::anchored_to_bottom(3, 1),
    );

    assert_eq!(renderer.presented.as_ref().unwrap().cursor_row, 2);
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

    let mut renderer = TestRenderer {
      presented: Some(PresentedFrame::new(
        frame.last_row(),
        frame,
        Viewport::anchored_to_bottom(1, 24),
      )),
      ..Default::default()
    };

    renderer
      .draw_frame(Frame::new(
        Vec::new(),
        Dimensions {
          height: 24,
          width: 80,
        },
      ))
      .unwrap();

    assert_eq!(
      String::from_utf8(renderer.stdout.clone()).unwrap(),
      "\x1b[?2026h\r\x1b[2K\x1b[?2026l",
    );

    assert_eq!(
      renderer.presented.as_ref().unwrap().frame.lines,
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

    let mut renderer = TestRenderer {
      presented: Some(PresentedFrame::new(
        frame.last_row(),
        frame,
        Viewport::anchored_to_bottom(3, 24),
      )),
      ..Default::default()
    };

    renderer.presented.as_mut().unwrap().cursor_row = 1;

    renderer.finish().unwrap();

    assert_eq!(
      String::from_utf8(renderer.stdout.clone()).unwrap(),
      "\x1b[1B"
    );
    assert_eq!(renderer.presented.as_ref().unwrap().cursor_row, 2);
  }

  #[test]
  fn full_render_clears_screen() {
    let mut renderer = TestRenderer::default();

    renderer
      .full_render(
        Frame::new(
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
      String::from_utf8(renderer.stdout.clone()).unwrap(),
      "\x1b[?2026h\x1b[2J\x1b[1;1H\x1b[3J\x1b[1G\x1b[2Kbar\r\n\x1b[1G\x1b[2Kbaz\x1b[?2026l",
    );
  }

  #[test]
  fn full_render_rebuilds_scrollback_when_clearing() {
    let mut renderer = TestRenderer::default();

    renderer
      .full_render(
        Frame::new(
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
      String::from_utf8(renderer.stdout.clone()).unwrap(),
      "\x1b[?2026h\x1b[2J\x1b[1;1H\x1b[3J\x1b[1G\x1b[2Kfoo\r\n\x1b[1G\x1b[2Kbar\r\n\x1b[1G\x1b[2Kbaz\x1b[?2026l",
    );
  }

  #[test]
  fn full_render_without_clear_prepares_lines() {
    let mut renderer = TestRenderer::default();

    renderer
      .full_render(
        Frame::new(
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
      String::from_utf8(renderer.stdout.clone()).unwrap(),
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

    let mut renderer = TestRenderer {
      presented: Some(PresentedFrame::new(
        frame.last_row(),
        frame,
        Viewport::anchored_to_bottom(5, 3),
      )),
      ..Default::default()
    };

    renderer
      .draw_frame(Frame::new(
        vec!["foo".into(), "bar".into(), "bob".into(), "qux".into()],
        Dimensions {
          height: 3,
          width: 80,
        },
      ))
      .unwrap();

    assert_eq!(
      String::from_utf8(renderer.stdout.clone()).unwrap(),
      "\x1b[?2026h\x1b[2A\r\x1b[2Kbob\r\n\x1b[2Kqux\r\n\x1b[2K\x1b[1A\x1b[?2026l",
    );

    assert_eq!(
      renderer.presented.as_ref().unwrap().viewport,
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

    let mut renderer = TestRenderer {
      presented: Some(PresentedFrame::new(
        frame.last_row(),
        frame,
        Viewport::anchored_to_bottom(3, 24),
      )),
      ..Default::default()
    };

    renderer
      .draw_frame(Frame::new(
        vec!["foo".into(), "qux".into(), "baz".into()],
        Dimensions {
          height: 24,
          width: 80,
        },
      ))
      .unwrap();

    assert_eq!(
      String::from_utf8(renderer.stdout.clone()).unwrap(),
      "\x1b[?2026h\x1b[1A\r\x1b[2Kqux\x1b[1B\x1b[?2026l",
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

    let mut renderer = TestRenderer {
      presented: Some(PresentedFrame::new(
        frame.last_row(),
        frame,
        Viewport::anchored_to_bottom(3, 2),
      )),
      ..Default::default()
    };

    renderer
      .draw_frame(Frame::new(
        vec!["qux".into(), "bar".into(), "baz".into()],
        Dimensions {
          height: 2,
          width: 80,
        },
      ))
      .unwrap();

    assert_eq!(
      String::from_utf8(renderer.stdout.clone()).unwrap(),
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

    let mut renderer = TestRenderer {
      presented: Some(PresentedFrame::new(
        frame.last_row(),
        frame,
        Viewport::anchored_to_bottom(1, 24),
      )),
      ..Default::default()
    };

    renderer
      .draw_frame(Frame::new(
        vec!["foo".into(), "bar".into()],
        Dimensions {
          height: 25,
          width: 80,
        },
      ))
      .unwrap();

    assert_eq!(
      String::from_utf8(renderer.stdout.clone()).unwrap(),
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

    let mut renderer = TestRenderer {
      presented: Some(PresentedFrame::new(
        frame.last_row(),
        frame.clone(),
        Viewport::anchored_to_bottom(5, 3),
      )),
      ..Default::default()
    };

    renderer
      .draw_frame(Frame::new(
        frame.lines,
        Dimensions {
          height: 4,
          width: 80,
        },
      ))
      .unwrap();

    assert_eq!(
      String::from_utf8(renderer.stdout.clone()).unwrap(),
      "\x1b[?2026h\x1b[2J\x1b[1;1H\x1b[3J\x1b[1G\x1b[2Kfoo\r\n\x1b[1G\x1b[2Kbar\r\n\x1b[1G\x1b[2Kbaz\r\n\x1b[1G\x1b[2Kqux\r\n\x1b[1G\x1b[2Kquux\x1b[?2026l",
    );

    assert_eq!(
      renderer.presented.as_ref().unwrap().viewport,
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

    let mut renderer = TestRenderer {
      presented: Some(PresentedFrame::new(
        frame.last_row(),
        frame,
        Viewport::anchored_to_bottom(1, 24),
      )),
      ..Default::default()
    };

    renderer
      .draw_frame(Frame::new(
        vec!["foo".into(), "bar".into()],
        Dimensions {
          height: 24,
          width: 81,
        },
      ))
      .unwrap();

    assert_eq!(
      String::from_utf8(renderer.stdout.clone()).unwrap(),
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

    let mut renderer = TestRenderer {
      presented: Some(PresentedFrame::new(
        frame.last_row(),
        frame,
        Viewport::anchored_to_bottom(3, 24),
      )),
      ..Default::default()
    };

    renderer
      .draw_frame(Frame::new(
        vec!["foo".into()],
        Dimensions {
          height: 24,
          width: 80,
        },
      ))
      .unwrap();

    assert_eq!(
      String::from_utf8(renderer.stdout.clone()).unwrap(),
      "\x1b[?2026h\x1b[1A\r\x1b[2K\r\n\x1b[2K\x1b[2A\x1b[?2026l",
    );
  }
}
