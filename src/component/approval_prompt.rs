use super::*;

#[derive(Debug)]
pub(crate) struct ApprovalPromptComponent<'a> {
  request: &'a ApprovalRequest,
}

impl<'a> ApprovalPromptComponent<'a> {
  pub(crate) fn new(request: &'a ApprovalRequest) -> Self {
    Self { request }
  }
}

impl Component for ApprovalPromptComponent<'_> {
  fn render(&self, width: u16) -> Vec<LineComponent> {
    [
      LineComponent::from([
        Span::styled("?", Style::Accent),
        Span::raw(" Approve "),
        Span::raw(self.request.invocation.to_string()),
        Span::raw("?"),
      ]),
      LineComponent::from([
        Span::styled("y", Style::Success),
        Span::styled(" approve · ", Style::Muted),
        Span::styled("n/esc", Style::Danger),
        Span::styled(" deny", Style::Muted),
      ]),
    ]
    .into_iter()
    .flat_map(|line| line.render(width))
    .collect()
  }
}
