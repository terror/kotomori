use super::*;

#[derive(Clone, Debug)]
pub(crate) struct Request<'a> {
  pub(crate) messages: &'a [Message],
  pub(crate) model: &'a Model,
  pub(crate) system: Option<&'a str>,
  pub(crate) tools: &'a [ToolDefinition],
}

impl Request<'_> {
  pub(crate) fn last_user_text(&self) -> Option<&str> {
    self.messages.iter().rev().find_map(Message::user_content)
  }
}
