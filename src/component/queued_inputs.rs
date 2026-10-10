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
            .push(
              LinesComponent::raw(input.split('\n'))
                .gutter(Span::styled("│ ", Style::Accent)),
            ),
        )
      })
      .render(width)
  }
}
