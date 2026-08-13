use super::*;

#[derive(Debug)]
pub(crate) struct QueuedInputsComponent<'a> {
  pub(super) inputs: &'a VecDeque<String>,
}

impl Component for QueuedInputsComponent<'_> {
  fn render(&self, width: u16) -> Vec<LineComponent> {
    if self.inputs.is_empty() {
      return Vec::new();
    }

    self
      .inputs
      .iter()
      .flat_map(|input| {
        once(LineComponent::from([Span::styled("Queued", Style::Muted)]))
          .chain(GutteredLinesComponent::raw(input.split('\n')).render(width))
          .chain(once(LineComponent::blank()))
      })
      .collect()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn rendering() {
    let inputs = VecDeque::from([
      "first follow-up".to_string(),
      "second\nfollow-up".to_string(),
    ]);

    assert_eq!(
      QueuedInputsComponent { inputs: &inputs }.render(80),
      [
        LineComponent::from([Span::styled("Queued", Style::Muted)]),
        LineComponent::from([
          Span::styled("│ ", Style::Accent),
          Span::raw("first follow-up"),
        ]),
        LineComponent::blank(),
        LineComponent::from([Span::styled("Queued", Style::Muted)]),
        LineComponent::from([
          Span::styled("│ ", Style::Accent),
          Span::raw("second"),
        ]),
        LineComponent::from([
          Span::styled("│ ", Style::Accent),
          Span::raw("follow-up"),
        ]),
        LineComponent::blank(),
      ]
    );
  }
}
