use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Line {
  spans: Vec<Span>,
}

impl Line {
  pub(crate) fn blank() -> Self {
    Self::raw("")
  }

  pub(crate) fn raw(text: impl Into<String>) -> Self {
    Self {
      spans: vec![Span::raw(text)],
    }
  }
}

impl Component for Line {
  fn render(&self, width: u16) -> Vec<Line> {
    let width = usize::from(width.max(1));

    let mut lines = Vec::new();
    let mut line = Vec::<Span>::new();

    let mut line_width = 0usize;

    for span in &self.spans {
      for c in span.text.chars() {
        let char_width = UnicodeWidthChar::width(c).unwrap_or(0);

        if line_width > 0 && line_width.saturating_add(char_width) > width {
          lines.push(Line { spans: line });

          line = Vec::new();
          line_width = 0;
        }

        match line.last_mut() {
          Some(last) if last.style == span.style => {
            last.text.push(c);
          }
          _ => line.push(Span {
            style: span.style,
            text: c.to_string(),
          }),
        }

        line_width = line_width.saturating_add(char_width);
      }
    }

    lines.push(Line { spans: line });

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
    line.spans
  }
}

impl From<Vec<Span>> for Line {
  fn from(spans: Vec<Span>) -> Self {
    Self { spans }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn displays_raw_text() {
    assert_eq!(Line::raw("foo").to_string(), "foo");
  }

  #[test]
  fn displays_styled_text() {
    assert_eq!(
      Line::from(vec![Span::styled("foo", Style::CyanBold)]).to_string(),
      "\x1b[36;1mfoo\x1b[0m",
    );
  }

  #[test]
  fn renders_raw_text_at_width() {
    assert_eq!(
      Line::raw("foobar").render(3),
      [Line::raw("foo"), Line::raw("bar")]
    );
  }

  #[test]
  fn renders_styled_text_at_width() {
    assert_eq!(
      Line::from(vec![Span::styled("foobar", Style::CyanBold)]).render(3),
      [
        Line::from(vec![Span::styled("foo", Style::CyanBold)]),
        Line::from(vec![Span::styled("bar", Style::CyanBold)]),
      ],
    );
  }

  #[test]
  fn renders_with_minimum_width_of_one() {
    assert_eq!(
      Line::raw("foo").render(0),
      [Line::raw("f"), Line::raw("o"), Line::raw("o")]
    );
  }

  #[test]
  fn rendering_preserves_style_boundaries() {
    assert_eq!(
      Line::from(vec![
        Span::styled("foo", Style::CyanBold),
        Span::styled("bar", Style::DarkGray),
      ])
      .render(6),
      [Line::from(vec![
        Span::styled("foo", Style::CyanBold),
        Span::styled("bar", Style::DarkGray),
      ])],
    );
  }
}
