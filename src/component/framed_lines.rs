use super::*;

#[derive(Debug, Clone)]
pub(crate) struct FramedLinesComponent {
  lines: Vec<LineComponent>,
}

impl FramedLinesComponent {
  pub(crate) fn new(lines: impl IntoIterator<Item = LineComponent>) -> Self {
    Self {
      lines: lines.into_iter().collect(),
    }
  }

  pub(crate) fn raw<'a>(lines: impl IntoIterator<Item = &'a str>) -> Self {
    Self::new(lines.into_iter().map(LineComponent::raw))
  }
}

impl Component for FramedLinesComponent {
  fn render(&self, width: u16) -> Vec<LineComponent> {
    let width = width.max(1);

    let border = "─".repeat(usize::from(width));

    let border_line =
      || LineComponent::from([Span::styled(border.clone(), Style::DarkGray)]);

    once(border_line())
      .chain(self.lines.iter().flat_map(|line| line.render(width)))
      .chain(once(border_line()))
      .collect()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn render_wraps_lines_between_borders() {
    assert_eq!(
      FramedLinesComponent::raw(["foobar"]).render(3),
      [
        LineComponent::from([Span::styled("───", Style::DarkGray)]),
        LineComponent::raw("foo"),
        LineComponent::raw("bar"),
        LineComponent::from([Span::styled("───", Style::DarkGray)]),
      ]
    );
  }
}
