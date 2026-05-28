use super::*;

mod transcript;

pub(crate) use transcript::TranscriptComponent;

pub(crate) trait Component {
  fn render(&self, width: u16) -> Vec<super::Line>;
}
