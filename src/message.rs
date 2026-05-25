use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Message {
  pub(crate) content: String,
  pub(crate) role: Role,
}

impl Message {
  pub(crate) fn lines(&self) -> Vec<Line> {
    match self.role {
      Role::Agent => self
        .content
        .split('\n')
        .map(|line| vec![Span::raw("  "), Span::raw(line.to_string())].into())
        .collect(),
      Role::User => self
        .content
        .split('\n')
        .map(|line| {
          vec![
            Span::raw("  "),
            Span::styled("❯ ", Style::CyanBold),
            Span::raw(line.to_string()),
          ]
          .into()
        })
        .collect(),
    }
  }

  pub(crate) fn new(role: Role, content: impl Into<String>) -> Self {
    Self {
      content: content.into(),
      role,
    }
  }
}
