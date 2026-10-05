use super::*;

#[derive(Debug)]
#[must_use]
pub(crate) struct ScrollComponent<'a, C> {
  pub(crate) component: C,
  pub(crate) focus: usize,
  pub(crate) header: Option<Box<dyn Component + 'a>>,
  pub(crate) height: usize,
  pub(crate) offset: &'a Cell<usize>,
}

impl<C: Component> Component for ScrollComponent<'_, C> {
  fn render(&self, width: u16) -> Vec<LineComponent> {
    if self.height == 0 {
      return Vec::new();
    }

    let mut lines = self
      .header
      .as_ref()
      .map_or_else(Vec::new, |header| header.render(width));

    let height = self.height.saturating_sub(1).max(1);

    if lines.len() > height {
      lines.retain(|line| !line.is_blank());

      lines.drain(..lines.len().saturating_sub(height));
    }

    let height = self.height.saturating_sub(lines.len());

    if height == 0 {
      return lines;
    }

    let rows = self.component.render(width);

    let focus = self.focus.min(rows.len().saturating_sub(1));

    let offset = self
      .offset
      .get()
      .min(focus)
      .max(focus.saturating_add(1).saturating_sub(height))
      .min(rows.len().saturating_sub(height));

    self.offset.set(offset);

    lines.extend(rows.into_iter().skip(offset).take(height));

    lines
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn follows_focus_and_clamps_after_resizing() {
    #[track_caller]
    fn case(
      offset: &Cell<usize>,
      focus: usize,
      height: usize,
      len: usize,
      expected: std::ops::Range<usize>,
    ) {
      assert_eq!(
        ScrollComponent {
          component: LinesComponent {
            lines: (0..len)
              .map(|index| LineComponent::raw(format!("foo{index}")))
              .collect()
          },
          focus,
          header: None,
          height,
          offset
        }
        .render(80),
        expected
          .map(|index| LineComponent::raw(format!("foo{index}")))
          .collect::<Vec<_>>(),
      );
    }

    let offset = Cell::new(0);

    case(&offset, 0, 3, 5, 0..3);
    case(&offset, 2, 3, 5, 0..3);
    case(&offset, 3, 3, 5, 1..4);
    case(&offset, 2, 3, 5, 1..4);
    case(&offset, 0, 3, 5, 0..3);
    case(&offset, 4, 3, 5, 2..5);
    case(&offset, 0, 3, 5, 0..3);
    case(&offset, 2, 0, 5, 0..0);
    case(&offset, 2, 3, 5, 0..3);
    case(&offset, 3, 1, 5, 3..4);
    case(&offset, 3, 4, 5, 1..5);
    case(&offset, 1, 4, 2, 0..2);
    case(&offset, 0, 4, 0, 0..0);
  }

  #[test]
  fn keeps_header_visible() {
    #[track_caller]
    fn case(height: usize, expected: &[&str]) {
      assert_eq!(
        ScrollComponent {
          component: LinesComponent::raw(["foo", "bar", "baz"]),
          focus: 2,
          header: Some(Box::new(
            StackComponent::default()
              .push_spaced(LineComponent::raw("qux"))
              .push(LineComponent::raw("quux")),
          )),
          height,
          offset: &Cell::new(0)
        }
        .render(80),
        expected
          .iter()
          .copied()
          .map(LineComponent::raw)
          .collect::<Vec<_>>(),
      );
    }

    case(0, &[]);
    case(1, &["quux"]);
    case(2, &["quux", "baz"]);
    case(3, &["qux", "quux", "baz"]);
    case(4, &["qux", "", "quux", "baz"]);
    case(5, &["qux", "", "quux", "bar", "baz"]);
    case(6, &["qux", "", "quux", "foo", "bar", "baz"]);
  }

  #[test]
  fn scrolls_wrapped_rows_inside_padding() {
    assert_eq!(
      PaddingComponent {
        component: ScrollComponent {
          component: LineComponent::from([Span::styled(
            "foobarbaz",
            Style::Accent
          )]),
          focus: 2,
          header: None,
          height: 2,
          offset: &Cell::new(0)
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
}
