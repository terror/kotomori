#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Style {
  CyanBold,
  DarkGray,
  Gray,
  None,
  Reverse,
}

impl Style {
  pub(crate) fn sequence(self) -> &'static str {
    match self {
      Self::CyanBold => "\x1b[36;1m",
      Self::DarkGray => "\x1b[90m",
      Self::Gray => "\x1b[37m",
      Self::None => "\x1b[0m",
      Self::Reverse => "\x1b[7m",
    }
  }
}
