#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Style {
  CyanBold,
  DarkGray,
  Gray,
  GreenBold,
  None,
  RedBold,
  Reverse,
}

impl Style {
  pub(crate) fn sequence(self) -> &'static str {
    match self {
      Self::CyanBold => "\x1b[36;1m",
      Self::DarkGray => "\x1b[90m",
      Self::Gray => "\x1b[37m",
      Self::GreenBold => "\x1b[32;1m",
      Self::None => "\x1b[0m",
      Self::RedBold => "\x1b[31;1m",
      Self::Reverse => "\x1b[7m",
    }
  }
}
