use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Message {
  pub(crate) content: String,
  pub(crate) role: Role,
}

impl Message {
  pub(crate) fn lines(&self, width: u16) -> Vec<Line> {
    match self.role {
      Role::Agent => self
        .content
        .split('\n')
        .map(|line| vec![Span::raw("  "), Span::raw(line.to_string())].into())
        .collect(),
      Role::User => {
        Composer::render_textarea_content(self.content.split('\n'), width)
      }
    }
  }

  pub(crate) fn new(role: Role, content: impl Into<String>) -> Self {
    Self {
      content: content.into(),
      role,
    }
  }
}
