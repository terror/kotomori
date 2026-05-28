use super::*;

mod header;
mod hint;
mod transcript;

pub(crate) use header::HeaderComponent;
pub(crate) use hint::HintComponent;
pub(crate) use transcript::TranscriptComponent;

pub(crate) trait Component {
  fn render(&self, width: u16) -> Vec<super::Line>;
}
