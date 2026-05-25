use super::*;

#[derive(Debug, Clone)]
pub(crate) struct FramedLines {
  lines: Vec<Line>,
}

impl FramedLines {
  pub(crate) fn new(lines: impl IntoIterator<Item = Line>) -> Self {
    Self {
      lines: lines.into_iter().collect(),
    }
  }

  pub(crate) fn raw<'a>(lines: impl IntoIterator<Item = &'a str>) -> Self {
    Self::new(lines.into_iter().map(Line::raw))
  }
}

impl Component for FramedLines {
  fn render(&self, width: u16) -> Vec<Line> {
    let width = width.max(1);

    let border = "─".repeat(usize::from(width));

    let mut rendered =
      vec![vec![Span::styled(border.clone(), Style::DarkGray)].into()];

    for line in &self.lines {
      rendered.extend(line.render(width));
    }

    rendered.push(vec![Span::styled(border, Style::DarkGray)].into());

    rendered
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn rendering() {
    assert_eq!(
      FramedLines::raw(["foobar"]).render(3),
      [
        vec![Span::styled("───", Style::DarkGray)].into(),
        Line::raw("foo"),
        Line::raw("bar"),
        vec![Span::styled("───", Style::DarkGray)].into(),
      ]
    );
  }
}
