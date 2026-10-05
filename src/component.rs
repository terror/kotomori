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
mod scroll;
mod stack;
mod text_field;
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
  resume_picker::ResumePickerComponent, scroll::ScrollComponent,
  stack::StackComponent, text_field::TextFieldComponent,
  transcript::TranscriptComponent, transcript_error::TranscriptErrorComponent,
  transcript_tool_invocation::TranscriptToolInvocationComponent,
  view::ViewComponent,
};

pub(crate) trait Component: Debug {
  fn gutter(self, gutter: impl Into<LineComponent>) -> GutterComponent<Self>
  where
    Self: Sized,
  {
    GutterComponent {
      component: self,
      gutter: gutter.into(),
    }
  }

  fn padded(self, padding: u16) -> PaddingComponent<Self>
  where
    Self: Sized,
  {
    PaddingComponent {
      component: self,
      padding,
    }
  }

  fn render(&self, width: u16) -> Vec<LineComponent>;

  fn scrolled(self, offset: usize, height: usize) -> ScrollComponent<Self>
  where
    Self: Sized,
  {
    ScrollComponent {
      component: self,
      height,
      offset,
    }
  }
}
