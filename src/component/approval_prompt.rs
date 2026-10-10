use super::*;

#[derive(Debug)]
pub(crate) struct ApprovalPromptComponent<'a> {
  pub(crate) request: &'a ApprovalRequest,
}

impl Component for ApprovalPromptComponent<'_> {
  fn render(&self, width: u16) -> Vec<LineComponent> {
    LinesComponent {
      lines: vec![
        LineComponent::from([
          Span::styled("?", Style::Accent),
          Span::raw(" Approve "),
          Span::raw(self.request.invocation.to_string()),
          Span::raw("?"),
        ]),
        LineComponent::from([
          Span::styled("y", Style::Success),
          Span::styled(" approve · ", Style::Muted),
          Span::styled("n/Esc", Style::Danger),
          Span::styled(" deny", Style::Muted),
        ]),
      ],
    }
    .render(width)
  }
}
