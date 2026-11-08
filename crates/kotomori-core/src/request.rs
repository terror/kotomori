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
}
