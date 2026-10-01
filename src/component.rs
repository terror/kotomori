use super::*;

mod approval_prompt;
mod composer;
mod footer;
mod guttered_lines;
mod header;
mod hint;
mod line;
mod queued_inputs;
mod resume_picker;
mod transcript;
mod transcript_error;
mod transcript_tool_invocation;
mod view;

pub(crate) use {
  approval_prompt::ApprovalPromptComponent, composer::ComposerComponent,
  footer::FooterComponent, guttered_lines::GutteredLinesComponent,
  header::HeaderComponent, hint::HintComponent, line::LineComponent,
  queued_inputs::QueuedInputsComponent, resume_picker::ResumePickerComponent,
  transcript::TranscriptComponent, transcript_error::TranscriptErrorComponent,
  transcript_tool_invocation::TranscriptToolInvocationComponent,
  view::ViewComponent,
};

pub(crate) trait Component {
  fn render(&self, width: u16) -> Vec<LineComponent>;
}

#[cfg(test)]
mod tests {
  use {super::*, unicode_width::UnicodeWidthStr};

  #[test]
  fn components_return_rows_within_their_width() {
    let settings = Settings {
      model: "mock:foo".parse().unwrap(),
      prompt: Some("/".into()),
      yolo: false,
    };
    let screen = Screen::Session(Box::new(
      State::new(Database::new().unwrap(), Session::new(&settings).unwrap())
        .unwrap(),
    ));
    let picker =
      Screen::Resume(ResumePicker::new(vec![Session::new(&settings).unwrap()]));
    let invocation = ToolInvocation::new(
      "foo",
      ToolInvocationKind::Command(CommandTool {
        command: "foobar".into(),
        cwd: Some("baz".into()),
      }),
    );
    let result = ToolResult {
      stdout: Some("foobar\nbar\nbaz\nqux".into()),
      ..Default::default()
    };
    let (request, _receiver) = ApprovalRequest::new(invocation.clone());
    let mut run = Run::new(0);
    run.update(MessageUpdate::ReasoningDelta {
      index: 0,
      delta: "foo界bar".into(),
    });
    let transcript = Transcript::with_entries(vec![
      TranscriptEntry::Message(Message::agent(vec![AssistantContent::text(
        "foo界bar",
      )])),
      TranscriptEntry::Notice("foobar".into()),
      TranscriptEntry::Interrupted,
    ]);

    for width in 0..=80 {
      for lines in [
        ApprovalPromptComponent::new(&request).render(width),
        ComposerComponent {
          composer: &Composer::new("/", Vec::new()),
        }
        .render(width),
        FooterComponent::new(&settings.model, Path::new("foobar"))
          .render(width),
        HeaderComponent::new(Some(Duration::from_millis(42))).render(width),
        HintComponent.render(width),
        QueuedInputsComponent {
          inputs: &VecDeque::from(["foobar".into()]),
        }
        .render(width),
        ResumePickerComponent::new(&ResumePicker::new(Vec::new()))
          .render(width),
        TranscriptComponent::new(&transcript, Some(&run)).render(width),
        TranscriptErrorComponent::new("foobar").render(width),
        TranscriptToolInvocationComponent::new(&invocation, Some(&result))
          .render(width),
        ViewComponent::new(&screen, None).render(width),
        ViewComponent::new(&picker, None).render(width),
      ] {
        for line in lines {
          let text = Vec::<Span>::from(line)
            .into_iter()
            .map(|span| span.text)
            .collect::<String>();
          assert!(
            UnicodeWidthStr::width(text.as_str()) <= usize::from(width),
            "{width}: {text:?}"
          );
        }
      }
    }
  }
}
