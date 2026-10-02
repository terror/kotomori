use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Style {
  Accent,
  Danger,
  Markdown(anstyle::Style),
  Muted,
  None,
  Secondary,
  Selection,
  Success,
}

impl Style {
  pub(crate) fn sequence(self) -> Cow<'static, str> {
    match self {
      Self::Accent => "\x1b[36;1m",
      Self::Danger => "\x1b[31;1m",
      Self::Markdown(style) => return style.to_string().into(),
      Self::Muted => "\x1b[90m",
      Self::None => "\x1b[0m",
      Self::Secondary => "\x1b[37m",
      Self::Selection => "\x1b[7m",
      Self::Success => "\x1b[32;1m",
    }
    .into()
  }
}

impl From<ratatui_core::style::Style> for Style {
  fn from(style: ratatui_core::style::Style) -> Self {
    let style = anstyle::Style::from(style);

    if style == anstyle::Style::new() {
      Self::None
    } else {
      Self::Markdown(style)
    }
  }
}
