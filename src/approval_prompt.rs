use super::*;

#[derive(Debug)]
pub(crate) struct ApprovalPrompt<'a> {
  request: &'a ApprovalRequest,
}

impl<'a> ApprovalPrompt<'a> {
  pub(crate) fn new(request: &'a ApprovalRequest) -> Self {
    Self { request }
  }
}

impl Component for ApprovalPrompt<'_> {
  fn render(&self, width: u16) -> Vec<Line> {
    FramedLines::new([
      vec![
        Span::styled("?", Style::CyanBold),
        Span::raw(" Approve "),
        Span::raw(self.request.invocation().to_string()),
        Span::raw("?"),
      ]
      .into(),
      vec![
        Span::styled("y", Style::GreenBold),
        Span::styled(" approve  ", Style::DarkGray),
        Span::styled("n", Style::RedBold),
        Span::styled(" deny  ", Style::DarkGray),
        Span::styled("esc", Style::RedBold),
        Span::styled(" deny", Style::DarkGray),
      ]
      .into(),
    ])
    .render(width)
  }
}
