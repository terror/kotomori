use super::*;

#[derive(Debug, Default)]
#[must_use]
pub(crate) struct StackComponent<'a> {
  children: Vec<Box<dyn Component + 'a>>,
  gap: usize,
}

impl<'a> StackComponent<'a> {
  pub(crate) fn gap(mut self, gap: usize) -> Self {
    self.gap = gap;

    self
  }

  pub(crate) fn push(mut self, component: impl Component + 'a) -> Self {
    self.children.push(Box::new(component));

    self
  }
}

impl Component for StackComponent<'_> {
  fn render(&self, width: u16) -> Vec<LineComponent> {
    let mut lines = Vec::new();

    for component in &self.children {
      let rows = component.render(width);

      if rows.is_empty() {
        continue;
      }

      if !lines.is_empty() {
        lines.resize_with(lines.len() + self.gap, LineComponent::blank);
      }

      lines.extend(rows);
    }

    lines
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn nested_stacks() {
    let stack = StackComponent::default()
      .push(LineComponent::blank())
      .push(
        StackComponent::default()
          .gap(1)
          .push(
            StackComponent::default()
              .gap(1)
              .push(LineComponent::raw("foo"))
              .push(LineComponent::raw("bar")),
          )
          .push(StackComponent::default())
          .push(
            StackComponent::default()
              .push(LineComponent::raw("baz"))
              .push(LineComponent::raw("qux")),
          ),
      )
      .push(LineComponent::blank());

    assert_eq!(
      stack.render(80),
      [
        LineComponent::blank(),
        LineComponent::raw("foo"),
        LineComponent::blank(),
        LineComponent::raw("bar"),
        LineComponent::blank(),
        LineComponent::raw("baz"),
        LineComponent::raw("qux"),
        LineComponent::blank(),
      ],
    );
  }

  #[test]
  fn spacing() {
    #[track_caller]
    fn case(gap: usize, children: &[&[&str]], expected: &[&str]) {
      let stack = children
        .iter()
        .fold(StackComponent::default().gap(gap), |stack, child| {
          stack.push(LinesComponent::raw(child.iter().copied()))
        });

      assert_eq!(
        stack.render(80),
        expected
          .iter()
          .map(|line| LineComponent::raw(*line))
          .collect::<Vec<_>>(),
      );
    }

    case(1, &[], &[]);
    case(1, &[&[], &[]], &[]);
    case(1, &[&[], &["foo"], &[]], &["foo"]);
    case(0, &[&["foo"], &["bar"]], &["foo", "bar"]);
    case(1, &[&["foo"], &[], &["bar"]], &["foo", "", "bar"]);
    case(2, &[&["foo"], &["bar"]], &["foo", "", "", "bar"]);
    case(
      1,
      &[&["", "foo", "", "", "bar", ""]],
      &["", "foo", "", "", "bar", ""],
    );
    case(
      1,
      &[&["foo", ""], &["", "bar"]],
      &["foo", "", "", "", "bar"],
    );
    case(1, &[&[""], &["foo"], &[""]], &["", "", "foo", "", ""]);
  }
}
