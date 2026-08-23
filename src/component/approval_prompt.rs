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
        Span::styled("n/Esc", Style::Danger),
        Span::styled(" deny", Style::Muted),
      ]),
    ]
    .into_iter()
    .flat_map(|line| line.render(width))
    .collect()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn escapes_control_characters_in_command() {
    let (request, _response_receiver) = ApprovalRequest::new(ToolInvocation {
      id: "foo".into(),
      kind: ToolInvocationKind::Command(CommandTool {
        command: "echo safe\r\x1b[2J\n? y approve".into(),
        cwd: None,
      }),
    });

    let line = ApprovalPromptComponent::new(&request).render(200).remove(0);

    let text = Vec::<Span>::from(line)
      .iter()
      .map(|span| span.text.as_str())
      .collect::<String>();

    assert_eq!(text, r"? Approve echo safe\r\u{1b}[2J\n? y approve?",);
  }
}
