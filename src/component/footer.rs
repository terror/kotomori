use super::*;

#[derive(Debug)]
pub(crate) struct FooterComponent<'a> {
  directory: &'a Path,
  model: &'a Model,
}

impl<'a> FooterComponent<'a> {
  pub(crate) fn new(model: &'a Model, directory: &'a Path) -> Self {
    Self { directory, model }
  }
}

impl Component for FooterComponent<'_> {
  fn render(&self, _width: u16) -> Vec<LineComponent> {
    let directory = self.directory.directory_display();

    vec![LineComponent::from([Span::styled(
      format!(
        "{} · {} · {directory}",
        self.model.provider, self.model.name
      ),
      Style::DarkGray,
    )])]
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn rendering() {
    assert_eq!(
      FooterComponent::new(
        &Model::new("foo", "bar").unwrap(),
        &PathBuf::from("baz")
      )
      .render(80),
      [LineComponent::from([Span::styled(
        "foo · bar · baz",
        Style::DarkGray
      )])]
    );
  }
}
