use super::*;

#[derive(Debug)]
pub(crate) struct ClipComponent<C> {
  component: C,
  height: usize,
}

impl<C> ClipComponent<C> {
  pub(crate) fn new(component: C, height: usize) -> Self {
    Self { component, height }
  }
}

impl<C: Component> Component for ClipComponent<C> {
  fn render(&self, width: u16) -> Vec<LineComponent> {
    if self.height == 0 {
      return Vec::new();
    }

    self
      .component
      .render(width)
      .into_iter()
      .take(self.height)
      .collect()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn clips_rendered_rows() {
    #[track_caller]
    fn case(height: usize, expected: &[LineComponent]) {
      assert_eq!(
        ClipComponent::new(
          LineComponent::from([
            Span::styled("foo", Style::Accent),
            Span::styled("bar", Style::Muted),
          ]),
          height,
        )
        .render(3),
        expected,
      );
    }

    let foo = LineComponent::from([Span::styled("foo", Style::Accent)]);
    let bar = LineComponent::from([Span::styled("bar", Style::Muted)]);

    case(0, &[]);
    case(1, slice::from_ref(&foo));
    case(2, &[foo.clone(), bar.clone()]);
    case(3, &[foo, bar]);
  }
}
