use super::*;

#[derive(Debug)]
#[must_use]
pub(crate) struct ScrollComponent {
  pub(crate) height: usize,
  pub(crate) offset: usize,
  pub(crate) rows: Vec<LineComponent>,
}

impl ScrollComponent {
  pub(crate) fn layout(self) -> Vec<LineComponent> {
    self
      .rows
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
      PaddingComponent { padding: 2 }.layout(7, |width| {
        ScrollComponent {
          height: 2,
          offset: 1,
          rows: LineComponent::styled("foobarbaz", Style::Accent).render(width),
        }
        .layout()
      }),
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
          height,
          offset,
          rows: (0..len)
            .map(|index| LineComponent::raw(format!("foo{index}")))
            .collect()
        }
        .layout(),
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
