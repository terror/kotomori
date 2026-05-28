use super::*;

mod approval_prompt;
mod footer;
mod framed_lines;
mod header;
mod hint;
mod transcript;
mod view;

pub(crate) use {
  approval_prompt::ApprovalPromptComponent, footer::FooterComponent,
  framed_lines::FramedLinesComponent, header::HeaderComponent,
  hint::HintComponent, transcript::TranscriptComponent, view::ViewComponent,
};

pub(crate) trait Component {
  fn render(&self, width: u16) -> Vec<super::Line>;
}
