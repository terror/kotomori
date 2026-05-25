use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Message {
  pub(crate) content: String,
  pub(crate) role: Role,
}

impl Message {
  pub(crate) fn new(role: Role, content: impl Into<String>) -> Self {
    Self {
      content: content.into(),
      role,
    }
  }
}

impl Component for Message {
  fn render(&self, width: u16) -> Vec<Line> {
    match self.role {
      Role::Agent => self.content.split('\n').map(Line::raw).collect(),
      Role::User => FramedLines::raw(self.content.split('\n')).render(width),
    }
  }
}
