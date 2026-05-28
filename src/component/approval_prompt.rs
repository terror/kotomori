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
        Span::styled("?", Style::CyanBold),
        Span::raw(" Approve "),
        Span::raw(self.request.invocation.to_string()),
        Span::raw("?"),
      ]),
      LineComponent::from([
        Span::styled("y", Style::GreenBold),
        Span::styled(" approve  ", Style::DarkGray),
        Span::styled("n", Style::RedBold),
        Span::styled(" deny  ", Style::DarkGray),
        Span::styled("esc", Style::RedBold),
        Span::styled(" deny", Style::DarkGray),
      ]),
    ]
    .into_iter()
    .flat_map(|line| line.render(width))
    .collect()
  }
}
