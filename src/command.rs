use super::*;

#[derive(Clone, Copy, Debug, EnumIter, Eq, PartialEq)]
pub(crate) enum Command {
  Clear,
  Compact,
  Quit,
}

impl Command {
  pub(crate) fn description(self) -> &'static str {
    match self {
      Self::Clear => "Clear the transcript",
      Self::Compact => "Compact the conversation context",
      Self::Quit => "Quit kotomori",
    }
  }

  pub(crate) fn from_input(input: &str) -> Option<Self> {
    let name = input.strip_prefix('/')?;

    if name.contains(char::is_whitespace) {
      return None;
    }

    Self::iter().find(|command| command.name() == name)
  }

  pub(crate) fn input(self) -> String {
    format!("/{}", self.name())
  }

  pub(crate) fn matches(self, input: &str) -> bool {
    let Some(name) = input.strip_prefix('/') else {
      return false;
    };

    !name.contains(char::is_whitespace) && self.name().starts_with(name)
  }

  pub(crate) fn name(self) -> &'static str {
    match self {
      Self::Clear => "clear",
      Self::Compact => "compact",
      Self::Quit => "quit",
    }
  }
}
