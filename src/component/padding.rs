use super::*;

#[derive(Debug)]
pub(crate) struct PaddingComponent<C> {
  component: C,
  padding: u16,
}

impl<C> PaddingComponent<C> {
  pub(crate) fn new(component: C, padding: u16) -> Self {
    Self { component, padding }
  }
}

impl<C: Component> Component for PaddingComponent<C> {
  fn render(&self, width: u16) -> Vec<LineComponent> {
    let padding = self.padding.min(width.saturating_sub(1) / 2);

    self
      .component
      .render(width - padding * 2)
      .into_iter()
      .map(|mut line| {
        if !line.is_blank() && padding > 0 {
          line
            .spans
            .insert(0, Span::raw(" ".repeat(usize::from(padding))));
        }

        line
      })
      .collect()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn composes_with_gutters_and_spacing() {
    let stack = StackComponent::default().push_spaced(GutterComponent::new(
      LinesComponent::raw(["foobar", ""]),
      Span::styled("│ ", Style::Accent),
    ));

    assert_eq!(
      PaddingComponent::new(stack, 2).render(9),
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
      ],
    );
  }

  #[test]
  fn shrinks_padding_to_fit() {
    #[track_caller]
    fn case(width: u16, expected: &[&str]) {
      assert_eq!(
        PaddingComponent::new(LineComponent::raw("foo"), 2)
          .render(width)
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
