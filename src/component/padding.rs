use super::*;

#[derive(Debug)]
pub(crate) struct PaddingComponent<C> {
  pub(crate) component: C,
  pub(crate) padding: u16,
}

impl<C: Component> Component for PaddingComponent<C> {
  fn render(&self, width: u16) -> Vec<LineComponent> {
    let padding = self.padding.min(width.saturating_sub(1) / 2);

    self
      .component
      .render(width - padding * 2)
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
      .push(GutterComponent {
        component: LinesComponent::raw(["foobar", ""]),
        gutter: LineComponent::styled("│ ", Style::Accent),
      })
      .push(LineComponent::raw("baz"));

    assert_eq!(
      PaddingComponent {
        component: stack,
        padding: 2
      }
      .render(9),
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
        PaddingComponent {
          component: LineComponent::raw("foo"),
          padding: 2
        }
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
