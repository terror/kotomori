use super::*;

#[derive(Debug)]
pub(crate) struct State {
  input: String,
  messages: Vec<Message>,
  options: Options,
  should_quit: bool,
}

impl State {
  pub(crate) fn handle_action(&mut self, action: Action) {
    match action {
      Action::Backspace => {
        self.input.pop();
      }
      Action::Input(c) => self.input.push(c),
      Action::None => {}
      Action::Quit => self.should_quit = true,
      Action::Submit => self.submit(),
    }
  }

  pub(crate) fn input(&self) -> &str {
    &self.input
  }

  pub(crate) fn messages(&self) -> &[Message] {
    &self.messages
  }

  pub(crate) fn new(options: Options) -> Self {
    let input = options.prompt.clone().unwrap_or_default();

    Self {
      input,
      messages: Vec::new(),
      options,
      should_quit: false,
    }
  }

  pub(crate) fn should_quit(&self) -> bool {
    self.should_quit
  }

  fn submit(&mut self) {
    let input = self.input.trim();

    if input.is_empty() {
      return;
    }

    let input = input.to_string();

    self.messages.push(Message::new(Role::User, input.clone()));

    self.messages.push(Message::new(
      Role::Agent,
      format!("queued for {}: {input}", self.options.model),
    ));

    self.input.clear();
  }

  pub(crate) fn transcript_height(&self, width: u16) -> u16 {
    let width = usize::from(width.max(1));

    self
      .messages
      .iter()
      .map(|message| {
        let len = message.width();

        u16::try_from(len.div_ceil(width).max(1)).unwrap_or(u16::MAX)
      })
      .fold(0u16, u16::saturating_add)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn options(prompt: Option<&str>) -> Options {
    Options {
      model: "foo".to_string(),
      prompt: prompt.map(str::to_string),
    }
  }

  #[test]
  fn backspace() {
    let mut state = State::new(options(Some("foo")));

    state.handle_action(Action::Backspace);

    assert_eq!(state.input(), "fo");
  }

  #[test]
  fn initializes_input_from_prompt() {
    assert_eq!(State::new(options(Some("foo"))).input(), "foo");
  }

  #[test]
  fn input() {
    let mut state = State::new(options(None));

    state.handle_action(Action::Input('f'));
    state.handle_action(Action::Input('o'));
    state.handle_action(Action::Input('o'));

    assert_eq!(state.input(), "foo");
  }

  #[test]
  fn none() {
    let mut state = State::new(options(Some("foo")));

    state.handle_action(Action::None);

    assert_eq!(state.input(), "foo");
    assert!(!state.should_quit());
    assert!(state.messages().is_empty());
  }

  #[test]
  fn quit() {
    let mut state = State::new(options(None));

    state.handle_action(Action::Quit);

    assert!(state.should_quit());
  }

  #[test]
  fn submit() {
    let mut state = State::new(options(Some(" foo ")));

    state.handle_action(Action::Submit);

    assert_eq!(state.input(), "");
    assert_eq!(
      state.messages(),
      [
        Message::new(Role::User, "foo"),
        Message::new(Role::Agent, "queued for foo: foo"),
      ]
    );
  }

  #[test]
  fn submit_empty_input() {
    let mut state = State::new(options(Some(" ")));

    state.handle_action(Action::Submit);

    assert_eq!(state.input(), " ");

    assert!(state.messages().is_empty());
  }

  #[test]
  fn transcript_height() {
    let mut state = State::new(options(Some("foo")));

    state.handle_action(Action::Submit);

    assert_eq!(state.transcript_height(10), 4);
  }
}
