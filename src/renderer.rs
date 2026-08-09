use super::*;

#[derive(Debug)]
pub(crate) struct Renderer<W: Write = BufWriter<Stdout>> {
  current: Option<Frame>,
  stdout: W,
}

impl Renderer {
  pub(crate) fn new() -> Result<Self> {
    enable_raw_mode().context("failed to enable raw mode")?;

    let mut stdout = BufWriter::new(io::stdout());

    queue!(stdout, Hide).context("failed to hide cursor")?;

    Ok(Self {
      current: None,
      stdout,
    })
  }
}

impl<W: Write> Renderer<W> {
  pub(crate) fn draw(&mut self, component: &impl Component) -> Result {
    let (width, height) =
      crossterm_terminal::size().context("failed to read terminal size")?;

    let lines = component
      .render(width)
      .into_iter()
      .flat_map(|line| line.render(width))
      .map(|line| format!("{line}{}", Style::None.sequence()))
      .collect::<Vec<_>>();

    self.draw_frame(Frame::new(
      lines,
      Dimensions {
        height: usize::from(height),
        width,
      },
    ))?;

    self.stdout.flush()?;

    Ok(())
  }

  fn draw_frame(&mut self, mut next: Frame) -> Result {
    let Some(current) = self.current.as_ref() else {
      self.stdout.begin_synchronized_update()?;
      self.stdout.write_lines(&next.lines)?;
      self.stdout.end_synchronized_update()?;

      self.current = Some(next);

      return Ok(());
    };

    let Some(plan) = RenderPlan::between(current, &next) else {
      return Ok(());
    };

    self.stdout.begin_synchronized_update()?;

    let current = match plan {
      RenderPlan::Full => {
        self.stdout.clear_screen()?;

        self.stdout.write_lines(&next.lines)?;

        next
      }
      RenderPlan::Patch(patch) => {
        self
          .stdout
          .move_to_row(current.last_row(), patch.move_target_row)?;

        if patch.prepend_line_feed {
          write!(self.stdout, "\r\n")?;
        }

        for row in patch.changed.first..=patch.changed.last {
          if row > patch.changed.first {
            write!(self.stdout, "\r\n")?;
          }

          self
            .stdout
            .replace_line(next.lines.get(row).map(String::as_str))?;
        }

        self
          .stdout
          .move_to_row(patch.changed.last, next.last_row())?;

        next.viewport_top = patch.viewport_top;
        next
      }
    };

    self.stdout.end_synchronized_update()?;

    self.current = Some(current);

    Ok(())
  }
}

impl<W: Default + Write> Default for Renderer<W> {
  fn default() -> Self {
    Self {
      current: None,
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
      current: Some(frame),
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

    assert_eq!(renderer.current.as_ref().unwrap().viewport_top, 1,);
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
      current: Some(frame),
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
      current: Some(frame),
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

    assert_eq!(renderer.current.as_ref().unwrap().viewport_top, 2,);
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
      current: Some(frame),
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
      renderer.current.as_ref().unwrap().lines,
      Vec::<String>::new(),
    );
  }

  #[test]
  fn full_render_clears_screen() {
    let mut renderer = TestRenderer {
      current: Some(Frame::new(
        Vec::new(),
        Dimensions {
          height: 9,
          width: 80,
        },
      )),
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
      current: Some(Frame::new(
        Vec::new(),
        Dimensions {
          height: 1,
          width: 80,
        },
      )),
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
      current: Some(frame),
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

    assert_eq!(renderer.current.as_ref().unwrap().viewport_top, 2,);
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
      current: Some(frame),
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
      current: Some(frame),
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
      current: Some(frame),
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
      current: Some(frame.clone()),
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

    assert_eq!(renderer.current.as_ref().unwrap().viewport_top, 1,);
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
      current: Some(frame),
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
      current: Some(frame),
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
