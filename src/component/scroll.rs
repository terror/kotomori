use super::*;

#[derive(Debug)]
#[must_use]
pub(crate) struct ScrollComponent<C> {
  pub(crate) component: C,
  pub(crate) height: usize,
  pub(crate) offset: usize,
}

impl<C: Component> Component for ScrollComponent<C> {
  fn render(&self, width: u16) -> Vec<LineComponent> {
    if self.height == 0 {
      return Vec::new();
    }

    self
      .component
      .render(width)
      .into_iter()
      .skip(self.offset)
      .take(self.height)
      .collect()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn scrolls_wrapped_rows_inside_padding() {
    assert_eq!(
      PaddingComponent {
        component: ScrollComponent {
          component: LineComponent::from([Span::styled(
            "foobarbaz",
            Style::Accent
          )]),
          height: 2,
          offset: 1
        },
        padding: 2
      }
      .render(7),
      [
        LineComponent::from([
          Span::raw("  "),
          Span::styled("bar", Style::Accent),
        ]),
        LineComponent::from([
          Span::raw("  "),
          Span::styled("baz", Style::Accent),
        ]),
      ],
    );
  }

  #[test]
  fn selects_rendered_rows() {
    #[track_caller]
    fn case(offset: usize, height: usize, len: usize, expected: Range<usize>) {
      assert_eq!(
        ScrollComponent {
          component: LinesComponent {
            lines: (0..len)
              .map(|index| LineComponent::raw(format!("foo{index}")))
              .collect()
          },
          height,
          offset
        }
        .render(80),
        expected
          .map(|index| LineComponent::raw(format!("foo{index}")))
          .collect::<Vec<_>>(),
      );
    }

    case(0, 0, 5, 0..0);
    case(0, 3, 5, 0..3);
    case(2, 3, 5, 2..5);
    case(4, 3, 5, 4..5);
    case(5, 3, 5, 5..5);
    case(usize::MAX, 3, 5, 5..5);
    case(0, 4, 2, 0..2);
    case(0, 4, 0, 0..0);
  }
}
