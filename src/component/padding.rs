use super::*;

#[derive(Debug)]
pub(crate) struct PaddingComponent {
  pub(crate) padding: u16,
}

impl PaddingComponent {
  pub(crate) fn layout(
    &self,
    width: u16,
    render: impl FnOnce(u16) -> Vec<LineComponent>,
  ) -> Vec<LineComponent> {
    let padding = self.padding.min(width.saturating_sub(1) / 2);

    render(width - padding * 2)
      .into_iter()
      .map(|line| {
        if line.is_blank() {
          line
        } else {
          LineComponent::raw(" ".repeat(usize::from(padding))).append(line)
        }
      })
      .collect()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn composes_with_gutters_and_spacing() {
    let stack = StackComponent::default()
      .gap(1)
      .push(
        LinesComponent::raw(["foobar", ""])
          .gutter(Span::styled("│ ", Style::Accent)),
      )
      .push(LineComponent::raw("baz"));

    assert_eq!(
      PaddingComponent { padding: 2 }.layout(9, |width| stack.render(width)),
      [
        LineComponent::from([
          Span::raw("  "),
          Span::styled("│ ", Style::Accent),
          Span::raw("foo"),
        ]),
        LineComponent::from([
          Span::raw("  "),
          Span::styled("│ ", Style::Accent),
          Span::raw("bar"),
        ]),
        LineComponent::from([
          Span::raw("  "),
          Span::styled("│ ", Style::Accent),
          Span::raw(""),
        ]),
        LineComponent::blank(),
        LineComponent::from([Span::raw("  "), Span::raw("baz")]),
      ],
    );
  }

  #[test]
  fn shrinks_padding_to_fit() {
    #[track_caller]
    fn case(width: u16, expected: &[&str]) {
      assert_eq!(
        PaddingComponent { padding: 2 }
          .layout(width, |width| LineComponent::raw("foo").render(width))
          .into_iter()
          .map(|line| line.to_string())
          .collect::<Vec<_>>(),
        expected,
      );
    }

    case(0, &[""]);
    case(1, &["f", "o", "o"]);
    case(2, &["fo", "o"]);
    case(3, &[" f", " o", " o"]);
    case(4, &[" fo", " o"]);
    case(5, &["  f", "  o", "  o"]);
    case(6, &["  fo", "  o"]);
    case(7, &["  foo"]);
  }
}
