use super::*;

#[derive(Debug)]
#[must_use]
pub(crate) struct ScrollComponent<'a, C> {
  component: C,
  focus: usize,
  header: Option<Box<dyn Component + 'a>>,
  height: usize,
  offset: &'a Cell<usize>,
}

impl<'a, C> ScrollComponent<'a, C> {
  pub(crate) fn header(mut self, component: impl Component + 'a) -> Self {
    self.header = Some(Box::new(component));

    self
  }

  pub(crate) fn new(
    component: C,
    height: usize,
    focus: usize,
    offset: &'a Cell<usize>,
  ) -> Self {
    Self {
      component,
      focus,
      header: None,
      height,
      offset,
    }
  }
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
        ScrollComponent::new(
          LinesComponent::new(
            (0..len).map(|index| LineComponent::raw(format!("foo{index}"))),
          ),
          height,
          focus,
          offset,
        )
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
        ScrollComponent::new(
          LinesComponent::raw(["foo", "bar", "baz"]),
          height,
          2,
          &Cell::new(0),
        )
        .header(
          StackComponent::default()
            .push_spaced(LineComponent::raw("qux"))
            .push(LineComponent::raw("quux")),
        )
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
      PaddingComponent::new(
        ScrollComponent::new(
          LineComponent::from([Span::styled("foobarbaz", Style::Accent)]),
          2,
          2,
          &Cell::new(0),
        ),
        2,
      )
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
