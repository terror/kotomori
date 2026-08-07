use super::*;

mod approval_prompt;
mod composer;
mod footer;
mod framed_lines;
mod header;
mod hint;
mod line;
mod transcript;
mod transcript_error;
mod transcript_tool_invocation;
mod view;

pub(crate) use {
  approval_prompt::ApprovalPromptComponent, composer::ComposerComponent,
  footer::FooterComponent, framed_lines::FramedLinesComponent,
  header::HeaderComponent, hint::HintComponent, line::LineComponent,
  transcript::TranscriptComponent, transcript_error::TranscriptErrorComponent,
  transcript_tool_invocation::TranscriptToolInvocationComponent,
  view::ViewComponent,
};

pub(crate) trait Component {
  fn render(&self, width: u16) -> Vec<LineComponent>;
}
