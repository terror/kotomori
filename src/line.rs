use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Line {
  spans: SmallVec<[Span; 6]>,
}

impl Line {
  pub(crate) fn blank() -> Self {
    Self::raw("")
  }

  pub(crate) fn raw(text: impl Into<String>) -> Self {
    Self {
      spans: [Span::raw(text)].into_iter().collect(),
    }
  }
}

impl Component for Line {
  fn render(&self, width: u16) -> Vec<Line> {
    let max_width = usize::from(width.max(1));

    let mut lines = Vec::new();

    let mut spans = SmallVec::<[Span; 6]>::new();
    let mut span_width = 0;

    for source_span in &self.spans {
      for c in source_span.text.chars() {
        let char_width = UnicodeWidthChar::width(c).unwrap_or(0);

        if span_width > 0 && span_width + char_width > max_width {
          lines.push(Line {
            spans: mem::take(&mut spans),
          });

          span_width = 0;
        }

        match spans.last_mut() {
          Some(last) if last.style == source_span.style => {
            last.text.push(c);
          }
          _ => spans.push(Span {
            style: source_span.style,
            text: c.to_string(),
          }),
        }

        span_width += char_width;
      }
    }

    lines.push(if spans.is_empty() {
      Line::blank()
    } else {
      Line { spans }
    });

    lines
  }
}

impl Display for Line {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    for span in &self.spans {
      if span.style == Style::None {
        write!(f, "{}", span.text)?;
      } else {
        write!(
          f,
          "{}{}{}",
          span.style.sequence(),
          span.text,
          Style::None.sequence()
        )?;
      }
    }

    Ok(())
  }
}

impl From<Line> for Vec<Span> {
  fn from(line: Line) -> Self {
    line.spans.into_vec()
  }
}

impl From<Vec<Span>> for Line {
  fn from(spans: Vec<Span>) -> Self {
    Self {
      spans: spans.into(),
    }
  }
}

impl<const N: usize> From<[Span; N]> for Line {
  fn from(spans: [Span; N]) -> Self {
    Self {
      spans: spans.into_iter().collect(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn displays_blank_line() {
    assert_eq!(Line::blank().to_string(), "");
  }

  #[test]
  fn displays_mixed_styled_and_raw_text() {
    assert_eq!(
      Line::from([
        Span::raw("a"),
        Span::styled("b", Style::CyanBold),
        Span::raw("c"),
      ])
      .to_string(),
      "a\x1b[36;1mb\x1b[0mc",
    );
  }

  #[test]
  fn displays_raw_text() {
    assert_eq!(Line::raw("foo").to_string(), "foo");
  }

  #[test]
  fn displays_styled_text() {
    assert_eq!(
      Line::from([Span::styled("foo", Style::CyanBold)]).to_string(),
      "\x1b[36;1mfoo\x1b[0m",
    );
  }

  #[test]
  fn line_with_six_spans_uses_inline_smallvec_storage() {
    assert!(
      !Line::from([
        Span::raw("a"),
        Span::raw("b"),
        Span::raw("c"),
        Span::raw("d"),
        Span::raw("e"),
        Span::raw("f"),
      ])
      .spans
      .spilled()
    );
  }

  #[test]
  fn rendering_accounts_for_wide_characters() {
    assert_eq!(
      Line::raw("a界b").render(3),
      [Line::raw("a界"), Line::raw("b")],
    );
  }

  #[test]
  fn rendering_keeps_zero_width_combining_marks_with_line() {
    assert_eq!(
      Line::raw("e\u{0301}x").render(1),
      [Line::raw("e\u{0301}"), Line::raw("x")],
    );
  }

  #[test]
  fn rendering_merges_adjacent_spans_with_same_style() {
    assert_eq!(
      Line::from([
        Span::styled("foo", Style::CyanBold),
        Span::styled("bar", Style::CyanBold),
      ])
      .render(6),
      [Line::from([Span::styled("foobar", Style::CyanBold)])],
    );
  }

  #[test]
  fn rendering_preserves_plain_text_content() {
    let line = Line::from([
      Span::raw("foo"),
      Span::styled("bar", Style::CyanBold),
      Span::raw("baz"),
    ]);

    let rendered_text = line
      .render(2)
      .into_iter()
      .flat_map(Vec::<Span>::from)
      .map(|span| span.text)
      .collect::<String>();

    assert_eq!(rendered_text, "foobarbaz");
  }

  #[test]
  fn rendering_preserves_style_boundaries() {
    assert_eq!(
      Line::from([
        Span::styled("foo", Style::CyanBold),
        Span::styled("bar", Style::DarkGray),
      ])
      .render(6),
      [Line::from([
        Span::styled("foo", Style::CyanBold),
        Span::styled("bar", Style::DarkGray),
      ])],
    );
  }

  #[test]
  fn renders_across_style_boundaries() {
    assert_eq!(
      Line::from([
        Span::styled("foo", Style::CyanBold),
        Span::styled("bar", Style::DarkGray),
      ])
      .render(4),
      [
        Line::from([
          Span::styled("foo", Style::CyanBold),
          Span::styled("b", Style::DarkGray),
        ]),
        Line::from([Span::styled("ar", Style::DarkGray)]),
      ],
    );
  }

  #[test]
  fn renders_blank_line() {
    assert_eq!(Line::blank().render(3), [Line::blank()]);
  }

  #[test]
  fn renders_raw_text_at_various_widths() {
    #[track_caller]
    fn case(width: u16, expected: &[&str]) {
      let expected =
        expected.iter().copied().map(Line::raw).collect::<Vec<_>>();

      assert_eq!(Line::raw("foo").render(width), expected);
    }

    case(0, &["f", "o", "o"]);
    case(1, &["f", "o", "o"]);
    case(2, &["fo", "o"]);
    case(3, &["foo"]);
    case(4, &["foo"]);
  }

  #[test]
  fn renders_styled_text_at_width() {
    assert_eq!(
      Line::from([Span::styled("foobar", Style::CyanBold)]).render(3),
      [
        Line::from([Span::styled("foo", Style::CyanBold)]),
        Line::from([Span::styled("bar", Style::CyanBold)]),
      ],
    );
  }

  #[test]
  fn renders_styled_text_that_exactly_fits_width() {
    assert_eq!(
      Line::from([Span::styled("foo", Style::CyanBold)]).render(3),
      [Line::from([Span::styled("foo", Style::CyanBold)])],
    );
  }
}
