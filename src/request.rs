use super::*;

#[derive(Debug, Clone)]
pub(crate) struct Request {
  messages: Vec<Message>,
  model: Model,
}

impl Request {
  pub(crate) fn last_user_message(&self) -> Option<&Message> {
    self
      .messages()
      .rev()
      .find(|message| message.role == Role::User)
  }

  pub(crate) fn messages(&self) -> impl DoubleEndedIterator<Item = &Message> {
    self.messages.iter()
  }

  pub(crate) fn model(&self) -> &Model {
    &self.model
  }

  pub(crate) fn model_name(&self) -> &str {
    self.model.name()
  }

  pub(crate) fn new(model: Model, messages: Vec<Message>) -> Self {
    Self { messages, model }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn chat_messages() {
    let request = Request::new(
      "fake:foo".parse().unwrap(),
      vec![
        Message::new(Role::User, "foo"),
        Message::new(Role::Agent, "bar"),
      ],
    );

    assert_eq!(request.model_name(), "foo");

    assert_eq!(
      request
        .messages()
        .map(|message| (message.role, message.content.as_str()))
        .collect::<Vec<_>>(),
      vec![(Role::User, "foo"), (Role::Agent, "bar")],
    );
  }

  #[test]
  fn last_user_message() {
    let request = Request::new(
      "fake:foo".parse().unwrap(),
      vec![
        Message::new(Role::User, "foo"),
        Message::new(Role::Agent, "bar"),
        Message::new(Role::User, "baz"),
      ],
    );

    assert_eq!(request.last_user_message().unwrap().content, "baz");
  }
}
