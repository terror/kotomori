use super::*;

#[derive(Debug)]
pub(crate) struct QueuedInputsComponent<'a> {
  pub(crate) inputs: &'a VecDeque<String>,
}

impl Component for QueuedInputsComponent<'_> {
  fn render(&self, width: u16) -> Vec<LineComponent> {
    self
      .inputs
      .iter()
      .fold(StackComponent::default().gap(1), |stack, input| {
        stack.push(
          StackComponent::default()
            .push(LineComponent::from([Span::styled("Queued", Style::Muted)]))
            .push(GutterComponent {
              component: LinesComponent::raw(input.split('\n')),
              gutter: Span::styled("│ ", Style::Accent),
            }),
        )
      })
      .render(width)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn rendering() {
    #[track_caller]
    fn case(inputs: &[&str], expected: &[LineComponent]) {
      let inputs = inputs.iter().map(|input| (*input).into()).collect();

      assert_eq!(
        QueuedInputsComponent { inputs: &inputs }.render(80),
        expected
      );
    }

    case(&[], &[]);

    case(
      &["foo", "bar\n\nbaz\n"],
      &[
        LineComponent::from([Span::styled("Queued", Style::Muted)]),
        LineComponent::from([
          Span::styled("│ ", Style::Accent),
          Span::raw("foo"),
        ]),
        LineComponent::blank(),
        LineComponent::from([Span::styled("Queued", Style::Muted)]),
        LineComponent::from([
          Span::styled("│ ", Style::Accent),
          Span::raw("bar"),
        ]),
        LineComponent::from([Span::styled("│ ", Style::Accent), Span::raw("")]),
        LineComponent::from([
          Span::styled("│ ", Style::Accent),
          Span::raw("baz"),
        ]),
        LineComponent::from([Span::styled("│ ", Style::Accent), Span::raw("")]),
      ],
    );
  }
}
