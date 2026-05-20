use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Message {
  pub(crate) content: String,
  pub(crate) role: Role,
}

impl Message {
  pub(crate) fn lines(&self) -> Vec<Line<'static>> {
    match self.role {
      Role::Agent => self
        .content
        .split('\n')
        .map(|line| {
          Line::from(vec![Span::raw("  "), Span::raw(line.to_string())])
        })
        .collect(),
      Role::User => self
        .content
        .split('\n')
        .map(|line| {
          Line::from(vec![
            Span::raw("  "),
            Span::styled(
              "❯ ",
              Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
            ),
            Span::raw(line.to_string()),
          ])
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

  pub(crate) fn width(&self) -> usize {
    match self.role {
      Role::Agent => 2 + self.content.len(),
      Role::User => 4 + self.content.len(),
    }
  }
}
