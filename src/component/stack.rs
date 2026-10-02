use super::*;

#[derive(Debug, Default)]
#[must_use]
pub(crate) struct StackComponent<'a> {
  children: Vec<(Box<dyn Component + 'a>, bool)>,
}

impl<'a> StackComponent<'a> {
  pub(crate) fn push(mut self, component: impl Component + 'a) -> Self {
    self.children.push((Box::new(component), false));

    self
  }

  pub(crate) fn push_spaced(mut self, component: impl Component + 'a) -> Self {
    self.children.push((Box::new(component), true));

    self
  }

  fn space(lines: &mut Vec<LineComponent>) {
    if lines.last().is_some_and(|line| !line.is_blank()) {
      lines.push(LineComponent::blank());
    }
  }
}

impl Component for StackComponent<'_> {
  fn render(&self, width: u16) -> Vec<LineComponent> {
    let mut lines = Vec::new();

    for (component, spaced) in &self.children {
      let rows = component.render(width);

      if rows.is_empty() {
        continue;
      }

      if *spaced {
        Self::space(&mut lines);
      }

      lines.extend(rows);

      if *spaced {
        Self::space(&mut lines);
      }
    }

    lines
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn spaces_nonempty_children_without_duplicate_blank_lines() {
    let stack = StackComponent::default()
      .push_spaced(LinesComponent::raw(["foo", ""]))
      .push_spaced(StackComponent::default())
      .push_spaced(LineComponent::raw("bar"))
      .push(LineComponent::raw("baz"))
      .push_spaced(LineComponent::raw("qux"));

    assert_eq!(
      stack.render(80),
      [
        LineComponent::raw("foo"),
        LineComponent::blank(),
        LineComponent::raw("bar"),
        LineComponent::blank(),
        LineComponent::raw("baz"),
        LineComponent::blank(),
        LineComponent::raw("qux"),
        LineComponent::blank(),
      ],
    );
  }
}
