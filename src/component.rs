use super::*;

mod approval_prompt;
mod composer;
mod footer;
mod framed_lines;
mod header;
mod hint;
mod message;
mod transcript;
mod transcript_tool_invocation;
mod view;

pub(crate) use {
  approval_prompt::ApprovalPromptComponent, composer::ComposerComponent,
  footer::FooterComponent, framed_lines::FramedLinesComponent,
  header::HeaderComponent, hint::HintComponent, message::MessageComponent,
  transcript::TranscriptComponent,
  transcript_tool_invocation::TranscriptToolInvocationComponent,
  view::ViewComponent,
};

pub(crate) trait Component {
  fn render(&self, width: u16) -> Vec<super::Line>;
}
