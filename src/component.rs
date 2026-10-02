use super::*;

mod approval_prompt;
mod composer;
mod footer;
mod gutter;
mod header;
mod hint;
mod line;
mod lines;
mod markdown;
mod output_preview;
mod padding;
mod queued_inputs;
mod resume_picker;
mod stack;
mod transcript;
mod transcript_error;
mod transcript_tool_invocation;
mod view;

pub(crate) use {
  approval_prompt::ApprovalPromptComponent, composer::ComposerComponent,
  footer::FooterComponent, gutter::GutterComponent, header::HeaderComponent,
  hint::HintComponent, line::LineComponent, lines::LinesComponent,
  markdown::MarkdownComponent, output_preview::OutputPreviewComponent,
  padding::PaddingComponent, queued_inputs::QueuedInputsComponent,
  resume_picker::ResumePickerComponent, stack::StackComponent,
  transcript::TranscriptComponent, transcript_error::TranscriptErrorComponent,
  transcript_tool_invocation::TranscriptToolInvocationComponent,
  view::ViewComponent,
};

pub(crate) trait Component: Debug {
  fn render(&self, width: u16) -> Vec<LineComponent>;
}
