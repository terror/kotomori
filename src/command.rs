use super::*;

#[derive(Clone, Copy, Debug, EnumIter, Eq, PartialEq)]
pub(crate) enum Command {
  Clear,
  Quit,
  Rename,
}

impl Command {
  fn accepts_arguments(self) -> bool {
    match self {
      Self::Clear | Self::Quit => false,
      Self::Rename => true,
    }
  }

  pub(crate) fn description(self) -> &'static str {
    match self {
      Self::Clear => "Clear the transcript",
      Self::Quit => "Quit kotomori",
      Self::Rename => "Rename the session",
    }
  }

  pub(crate) fn from_input(input: &str) -> Option<(Self, &str)> {
    let input = input.strip_prefix('/')?;

    let (name, arguments) =
      input.split_once(char::is_whitespace).unwrap_or((input, ""));

    let command = Self::iter().find(|command| command.name() == name)?;
    let arguments = arguments.trim();

    (command.accepts_arguments() || arguments.is_empty())
      .then_some((command, arguments))
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
      Self::Quit => "quit",
      Self::Rename => "rename",
    }
  }
}
