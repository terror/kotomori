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
    let Some(presented) = self.presented.as_ref() else {
      self.stdout.begin_synchronized_update()?;
      self.stdout.write_lines(&next.lines)?;
      self.stdout.end_synchronized_update()?;

      self.presented = Some(next.into());

      return Ok(());
    };

    let Some(plan) = RenderPlan::between(presented, &next) else {
      return Ok(());
    };

    self.stdout.begin_synchronized_update()?;

    let presented = match plan {
      RenderPlan::Full => {
        self.stdout.clear_screen()?;
        self.stdout.write_lines(&next.lines)?;

        next.into()
      }
      RenderPlan::Patch(changed) => {
        let viewport_top = self.patch_render(&next, changed)?;

        PresentedFrame::new(next, viewport_top)
      }
    };

    self.stdout.end_synchronized_update()?;

    self.presented = Some(presented);

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

  fn line_feed(
    &mut self,
    cursor_row: &mut usize,
    viewport_top: &mut usize,
    viewport_height: usize,
  ) -> Result {
    write!(self.stdout, "\r\n")?;

    if cursor_row.saturating_sub(*viewport_top)
      >= viewport_height.saturating_sub(1)
    {
      *viewport_top = viewport_top.saturating_add(1);
    }

    *cursor_row = cursor_row.saturating_add(1);

    Ok(())
  }

  fn patch_render(
    &mut self,
    next: &Frame,
    changed: ChangedRange,
  ) -> Result<usize> {
    let presented = self.presented.as_ref().unwrap();

    let (mut cursor_row, mut viewport_top) =
      (presented.frame.last_row(), presented.viewport_top);

    let append_start =
      changed.first > 0 && changed.first == presented.frame.len();

    let move_target_row = if append_start {
      changed.first.saturating_sub(1)
    } else {
      changed.first
    };

    let viewport_bottom =
      viewport_top.saturating_add(next.dimensions.height.saturating_sub(1));

    debug_assert!(move_target_row <= viewport_bottom);

    self.stdout.move_to_row(cursor_row, move_target_row)?;

    cursor_row = move_target_row;

    if append_start {
      self.line_feed(
        &mut cursor_row,
        &mut viewport_top,
        next.dimensions.height,
      )?;
    }

    for row in changed.first..=changed.last {
      if row > changed.first {
        self.line_feed(
          &mut cursor_row,
          &mut viewport_top,
          next.dimensions.height,
        )?;
      }

      self
        .stdout
        .replace_line(next.lines.get(row).map(String::as_str))?;
    }

    let last_row = next.last_row();

    self.stdout.move_to_row(cursor_row, last_row)?;

    Ok(viewport_top)
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
      presented: Some(frame.into()),
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
      "\x1b[?2026h\r\n\x1b[1G\x1b[2K\x1b[?2026l",
    );

    assert_eq!(renderer.presented.as_ref().unwrap().viewport_top, 1,);
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
      presented: Some(frame.into()),
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
      "\x1b[?2026h\r\n\x1b[1G\x1b[2Kbar\x1b[?2026l",
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
      presented: Some(frame.into()),
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
      "\x1b[?2026h\r\n\x1b[1G\x1b[2Kbaz\x1b[?2026l",
    );

    assert_eq!(renderer.presented.as_ref().unwrap().viewport_top, 2,);
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
      presented: Some(frame.into()),
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
      "\x1b[?2026h\x1b[1G\x1b[2K\x1b[?2026l",
    );

    assert_eq!(
      renderer.presented.as_ref().unwrap().frame.lines,
      Vec::<String>::new(),
    );
  }

  #[test]
  fn full_render_clears_screen() {
    let mut renderer = TestRenderer {
      presented: Some(
        Frame::new(
          Vec::new(),
          Dimensions {
            height: 9,
            width: 80,
          },
        )
        .into(),
      ),
      ..Default::default()
    };

    renderer
      .draw_frame(Frame::new(
        vec!["bar".into(), "baz".into()],
        Dimensions {
          height: 10,
          width: 80,
        },
      ))
      .unwrap();

    assert_eq!(
      String::from_utf8(renderer.stdout.clone()).unwrap(),
      "\x1b[?2026h\x1b[2J\x1b[1;1H\x1b[3J\x1b[1G\x1b[2Kbar\r\n\x1b[1G\x1b[2Kbaz\x1b[?2026l",
    );
  }

  #[test]
  fn full_render_rebuilds_scrollback_when_clearing() {
    let mut renderer = TestRenderer {
      presented: Some(
        Frame::new(
          Vec::new(),
          Dimensions {
            height: 1,
            width: 80,
          },
        )
        .into(),
      ),
      ..Default::default()
    };

    renderer
      .draw_frame(Frame::new(
        vec!["foo".into(), "bar".into(), "baz".into()],
        Dimensions {
          height: 2,
          width: 80,
        },
      ))
      .unwrap();

    assert_eq!(
      String::from_utf8(renderer.stdout.clone()).unwrap(),
      "\x1b[?2026h\x1b[2J\x1b[1;1H\x1b[3J\x1b[1G\x1b[2Kfoo\r\n\x1b[1G\x1b[2Kbar\r\n\x1b[1G\x1b[2Kbaz\x1b[?2026l",
    );
  }

  #[test]
  fn initial_render_prepares_lines_without_clearing() {
    let mut renderer = TestRenderer::default();

    renderer
      .draw_frame(Frame::new(
        vec!["foo".into(), "bar".into()],
        Dimensions {
          height: 10,
          width: 80,
        },
      ))
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
      presented: Some(frame.into()),
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
      "\x1b[?2026h\x1b[2A\x1b[1G\x1b[2Kbob\r\n\x1b[1G\x1b[2Kqux\r\n\x1b[1G\x1b[2K\x1b[1A\x1b[?2026l",
    );

    assert_eq!(renderer.presented.as_ref().unwrap().viewport_top, 2,);
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
      presented: Some(frame.into()),
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
      "\x1b[?2026h\x1b[1A\x1b[1G\x1b[2Kqux\x1b[1B\x1b[?2026l",
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
      presented: Some(frame.into()),
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
      presented: Some(frame.into()),
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
      presented: Some(frame.clone().into()),
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

    assert_eq!(renderer.presented.as_ref().unwrap().viewport_top, 1,);
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
      presented: Some(frame.into()),
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
      presented: Some(frame.into()),
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
      "\x1b[?2026h\x1b[1A\x1b[1G\x1b[2K\r\n\x1b[1G\x1b[2K\x1b[2A\x1b[?2026l",
    );
  }
}
