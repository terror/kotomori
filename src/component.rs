use super::*;

pub(crate) trait Component {
  fn render(&self, width: u16) -> Vec<Line>;
}
