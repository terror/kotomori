use super::*;

mod framed_lines;
mod header;
mod hint;
mod transcript;

pub(crate) use framed_lines::FramedLinesComponent;
pub(crate) use header::HeaderComponent;
pub(crate) use hint::HintComponent;
pub(crate) use transcript::TranscriptComponent;

pub(crate) trait Component {
  fn render(&self, width: u16) -> Vec<super::Line>;
}
