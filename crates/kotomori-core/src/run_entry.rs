use super::*;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum RunEntry {
  Draft(MessageBuffer),
  Message(Message),
}

impl RunEntry {
  #[must_use]
  pub fn into_message(self) -> Option<Message> {
    let message = match self {
      Self::Draft(buffer) => Message::Agent(buffer.finish()),
      Self::Message(message) => message,
    };

    match message {
      Message::Agent(message) if message.content.is_empty() => None,
      message => Some(message),
    }
  }
}
