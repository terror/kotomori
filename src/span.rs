use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Span {
  pub(crate) style: Style,
  pub(crate) text: String,
}

impl Span {
  pub(crate) fn raw(text: impl Into<String>) -> Self {
    Self {
      style: Style::None,
      text: text.into(),
    }
  }

  pub(crate) fn styled(text: impl Into<String>, style: Style) -> Self {
    Self {
      style,
      text: text.into(),
    }
  }
}
