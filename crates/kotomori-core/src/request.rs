use super::*;

#[derive(Clone, Debug)]
pub struct Request<'a> {
  pub messages: &'a [Message],
  pub model: &'a Model,
  pub system: Option<&'a str>,
  pub tools: &'a [ToolDefinition],
}

impl Request<'_> {
  pub fn last_user_text(&self) -> Option<&str> {
    self.messages.iter().rev().find_map(Message::user_content)
  }

  pub(super) fn validate(&self) -> Result {
    let mut pending = Vec::<&ToolRequest>::new();

    for message in self.messages {
      match message {
        Message::Agent(message) => {
          ensure!(pending.is_empty(), "incomplete tool exchange in context");

          for content in &message.content {
            if let AssistantContent::ToolCall(call) = content {
              ensure!(
                !pending.iter().any(|pending| pending.id == call.id),
                "duplicate tool call in context"
              );

              pending.push(call);
            }
          }
        }
        Message::User(content) => {
          for content in content {
            if matches!(content, UserMessageContent::Text(_)) {
              ensure!(
                pending.is_empty(),
                "incomplete tool exchange in context"
              );
            }

            if let UserMessageContent::ToolResult { call, name, .. } = content {
              let index = pending
                .iter()
                .position(|pending| {
                  pending.id == *call && pending.function.name == *name
                })
                .context("tool result without a matching call in context")?;

              pending.remove(index);
            }
          }
        }
      }
    }

    ensure!(pending.is_empty(), "incomplete tool exchange in context");

    Ok(())
  }
}
