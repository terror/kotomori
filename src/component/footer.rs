use super::*;

#[derive(Debug)]
pub(crate) struct FooterComponent<'a> {
  pub(crate) directory: &'a Path,
  pub(crate) model: &'a Model,
}

impl Component for FooterComponent<'_> {
  fn render(&self, width: u16) -> Vec<LineComponent> {
    let directory = DirectoryDisplay::new(self.directory);

    LineComponent::from([Span::styled(
      format!(
        "{} · {} · {directory}",
        self.model.provider, self.model.name
      ),
      Style::Muted,
    )])
    .render(width)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn rendering() {
    assert_eq!(
      FooterComponent {
        directory: &PathBuf::from("baz"),
        model: &Model::new("foo", "bar").unwrap()
      }
      .render(80),
      [LineComponent::from([Span::styled(
        "foo · bar · baz",
        Style::Muted
      )])]
    );
  }
}
