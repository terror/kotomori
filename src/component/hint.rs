use super::*;

#[derive(Debug)]
pub(crate) struct HintComponent;

impl Component for HintComponent {
  fn render(&self, _width: u16) -> Vec<LineComponent> {
    vec![LineComponent::from([
      Span::styled("Type a prompt. Press ", Style::DarkGray),
      Span::styled("Ctrl-C", Style::Gray),
      Span::styled(" to quit.", Style::DarkGray),
    ])]
  }
}
