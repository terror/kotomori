use super::*;

#[derive(Debug)]
pub(crate) struct HintComponent;

impl Component for HintComponent {
  fn render(&self, width: u16) -> Vec<LineComponent> {
    LineComponent::from([
      Span::styled("Type a prompt. Press ", Style::Muted),
      Span::styled("Ctrl-C", Style::Secondary),
      Span::styled(" to quit.", Style::Muted),
    ])
    .wrap(width)
  }
}
