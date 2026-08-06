use super::*;

#[derive(Debug)]
pub(crate) struct MessageComponent<'a> {
  message: &'a Message,
}

impl<'a> MessageComponent<'a> {
  pub(crate) fn new(message: &'a Message) -> Self {
    Self { message }
  }
}

impl Component for MessageComponent<'_> {
  fn render(&self, width: u16) -> Vec<LineComponent> {
    match self.message {
      Message::Agent(content) => content
        .iter()
        .flat_map(|content| match content {
          AgentMessageContent::Reasoning(reasoning) => reasoning
            .split('\n')
            .map(|line| {
              LineComponent::from([Span::styled(
                format!(" {line}"),
                Style::DarkGray,
              )])
            })
            .collect::<Vec<_>>(),
          AgentMessageContent::Text(text) => {
            text.split('\n').map(LineComponent::raw).collect::<Vec<_>>()
          }
          AgentMessageContent::ToolCall(invocation) => {
            vec![LineComponent::raw(invocation.to_string())]
          }
        })
        .collect(),
      Message::User(content) => content
        .iter()
        .flat_map(|content| match content {
          UserMessageContent::Text(text) => {
            FramedLinesComponent::raw(text.split('\n')).render(width)
          }
          UserMessageContent::ToolResult { result, .. } => result
            .output()
            .unwrap_or_default()
            .split('\n')
            .map(LineComponent::raw)
            .collect::<Vec<_>>(),
        })
        .collect(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn render_agent_content() {
    let invocation = ToolInvocation {
      id: "foo".into(),
      kind: ToolInvocationKind::Command(CommandTool {
        arguments: vec!["--files".into()],
        cwd: Some("bar".into()),
        program: "rg".into(),
      }),
    };

    let message = Message::Agent(vec![
      AgentMessageContent::Reasoning("foo\nbar".into()),
      AgentMessageContent::Text("baz\nqux".into()),
      AgentMessageContent::ToolCall(invocation),
    ]);

    assert_eq!(
      MessageComponent::new(&message).render(80),
      [
        LineComponent::from([Span::styled(" foo", Style::DarkGray)]),
        LineComponent::from([Span::styled(" bar", Style::DarkGray)]),
        LineComponent::raw("baz"),
        LineComponent::raw("qux"),
        LineComponent::raw("rg --files"),
      ]
    );
  }

  #[test]
  fn render_user_content() {
    let message = Message::User(vec![
      UserMessageContent::Text("foobar".into()),
      UserMessageContent::ToolResult {
        id: "foo".into(),
        result: ToolResult::command(Some(0), "bar\nbaz", "qux"),
      },
    ]);

    assert_eq!(
      MessageComponent::new(&message).render(3),
      [
        LineComponent::from([Span::styled("───", Style::DarkGray)]),
        LineComponent::raw("foo"),
        LineComponent::raw("bar"),
        LineComponent::from([Span::styled("───", Style::DarkGray)]),
        LineComponent::raw("bar"),
        LineComponent::raw("baz"),
        LineComponent::raw("qux"),
      ]
    );
  }
}
