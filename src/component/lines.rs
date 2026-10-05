use super::*;

#[derive(Debug)]
pub(crate) struct LinesComponent {
  pub(crate) lines: Vec<LineComponent>,
}

impl LinesComponent {
  pub(crate) fn raw<'a>(lines: impl IntoIterator<Item = &'a str>) -> Self {
    Self {
      lines: lines.into_iter().map(LineComponent::raw).collect(),
    }
  }
}

impl Component for LinesComponent {
  fn render(&self, width: u16) -> Vec<LineComponent> {
    self
      .lines
      .iter()
      .flat_map(|line| line.render(width))
      .collect()
  }
}
