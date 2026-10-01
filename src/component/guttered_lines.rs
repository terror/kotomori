use super::*;

#[derive(Clone, Debug)]
pub(crate) struct GutteredLinesComponent {
  gutter: Span,
  lines: Vec<LineComponent>,
}

impl GutteredLinesComponent {
  pub(crate) fn new(lines: impl IntoIterator<Item = LineComponent>) -> Self {
    Self {
      gutter: Span::styled("│ ", Style::Accent),
      lines: lines.into_iter().collect(),
    }
  }

  pub(crate) fn raw<'a>(lines: impl IntoIterator<Item = &'a str>) -> Self {
    Self::new(lines.into_iter().map(LineComponent::raw))
  }

  pub(crate) fn with_gutter(mut self, gutter: Span) -> Self {
    self.gutter = gutter;
    self
  }
}

impl Component for GutteredLinesComponent {
  fn render(&self, width: u16) -> Vec<LineComponent> {
    let mut remaining = usize::from(width.saturating_sub(1));

    let gutter = self
      .gutter
      .text
      .chars()
      .map_while(|c| {
        remaining =
          remaining.checked_sub(UnicodeWidthChar::width(c).unwrap_or(0))?;
        Some(c)
      })
      .collect::<String>();

    let content_width = u16::try_from(remaining).unwrap() + width.min(1);

    self
      .lines
      .iter()
      .flat_map(|line| line.wrap(content_width))
      .map(|line| {
        let mut spans = Vec::<Span>::from(line);
        if !gutter.is_empty() {
          spans.insert(0, Span::styled(&gutter, self.gutter.style));
        }
        LineComponent::from(spans)
      })
      .collect()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn narrow_gutters_leave_room_for_content() {
    #[track_caller]
    fn case(width: u16, expected: &[LineComponent]) {
      assert_eq!(GutteredLinesComponent::raw(["foo"]).render(width), expected);
    }

    case(0, &[LineComponent::blank()]);
    case(
      1,
      &[
        LineComponent::raw("f"),
        LineComponent::raw("o"),
        LineComponent::raw("o"),
      ],
    );
    case(
      2,
      &[
        LineComponent::from([Span::styled("│", Style::Accent), Span::raw("f")]),
        LineComponent::from([Span::styled("│", Style::Accent), Span::raw("o")]),
        LineComponent::from([Span::styled("│", Style::Accent), Span::raw("o")]),
      ],
    );
  }

  #[test]
  fn render_wraps_lines_with_an_accent_gutter() {
    assert_eq!(
      GutteredLinesComponent::raw(["foobar"]).render(5),
      [
        LineComponent::from([
          Span::styled("│ ", Style::Accent),
          Span::raw("foo"),
        ]),
        LineComponent::from([
          Span::styled("│ ", Style::Accent),
          Span::raw("bar"),
        ]),
      ]
    );
  }
}
