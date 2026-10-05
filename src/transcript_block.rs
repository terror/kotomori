use super::*;

#[derive(Debug, PartialEq)]
pub(crate) enum TranscriptBlock<'a> {
  Markdown(&'a str),
  Raw(String),
  Reasoning(Cow<'a, str>),
  Tool {
    invocation: ToolInvocation,
    result: Option<&'a ToolResult>,
  },
}

impl<'a> TranscriptBlock<'a> {
  pub(crate) fn content(
    content: &'a [AssistantContent],
    following: &'a [TranscriptEntry],
  ) -> impl Iterator<Item = Self> {
    content
      .iter()
      .filter_map(move |content| Self::from_content(content, following))
  }

  pub(crate) fn draft(
    buffer: &'a MessageBuffer,
    following: &'a [TranscriptEntry],
  ) -> impl Iterator<Item = Self> {
    buffer.preview().filter_map(move |block| match block {
      MessageDraft::Content(content) => Self::from_content(content, following),
      MessageDraft::Reasoning(reasoning) => {
        Some(Self::Reasoning(Cow::Borrowed(reasoning)))
      }
    })
  }

  fn from_content(
    content: &'a AssistantContent,
    following: &'a [TranscriptEntry],
  ) -> Option<Self> {
    match content {
      AssistantContent::Reasoning(reasoning) => {
        Some(Self::Reasoning(Cow::Owned(reasoning.text())))
      }
      AssistantContent::Text(text) => Some(Self::Markdown(&text.text)),
      AssistantContent::ToolCall(call) => {
        Some(match ToolInvocationKind::decode(call.clone()) {
          Ok(invocation) => Self::Tool {
            invocation,
            result: following
              .iter()
              .map_while(|entry| match entry {
                TranscriptEntry::Draft(_)
                | TranscriptEntry::Message(Message::Agent(_)) => None,
                TranscriptEntry::Message(Message::User(content)) => {
                  Some(content.as_slice())
                }
                TranscriptEntry::Error(_)
                | TranscriptEntry::Interrupted
                | TranscriptEntry::Notice(_) => Some(&[]),
              })
              .flatten()
              .find_map(|content| match content {
                UserMessageContent::ToolResult {
                  call: id, result, ..
                } if *id == call.id => Some(result),
                _ => None,
              }),
          },
          Err(_) => Self::Raw(format!(
            "{} {}",
            call.function.name, call.function.arguments
          )),
        })
      }
      AssistantContent::Image(_) => None,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn tool_results() {
    #[track_caller]
    fn case(following: &[TranscriptEntry], result: Option<&ToolResult>) {
      let invocation = ToolInvocation::new(
        "foo",
        ToolInvocationKind::Command(CommandTool {
          command: "bar".into(),
          cwd: None,
        }),
      );

      let (content, expected) = (
        [AssistantContent::ToolCall(invocation.protocol.clone())],
        [TranscriptBlock::Tool { invocation, result }],
      );

      assert_eq!(
        TranscriptBlock::content(&content, following).collect::<Vec<_>>(),
        expected,
      );

      let mut buffer = MessageBuffer::default();

      buffer.apply(MessageUpdate::Complete(content.to_vec().into()));

      assert_eq!(
        TranscriptBlock::draft(&buffer, following).collect::<Vec<_>>(),
        expected,
      );
    }

    let result = ToolResult {
      content: Some("baz".into()),
      outcome: ToolOutcome::Success,
      ..Default::default()
    };

    let entry = TranscriptEntry::Message(Message::User(vec![
      UserMessageContent::Text("foo".into()),
      UserMessageContent::ToolResult {
        call: CallId::from_wire("qux"),
        name: ToolName::new("command").unwrap(),
        result: ToolResult::default(),
      },
      UserMessageContent::ToolResult {
        call: CallId::from_wire("foo"),
        name: ToolName::new("command").unwrap(),
        result: result.clone(),
      },
      UserMessageContent::ToolResult {
        call: CallId::from_wire("foo"),
        name: ToolName::new("command").unwrap(),
        result: ToolResult::default(),
      },
    ]));

    case(&[], None);

    case(
      &[
        TranscriptEntry::Message(Message::User(vec![
          UserMessageContent::Text("foo".into()),
        ])),
        TranscriptEntry::Error("foo".into()),
        TranscriptEntry::Notice("bar".into()),
        TranscriptEntry::Interrupted,
        entry.clone(),
      ],
      Some(&result),
    );

    for boundary in [
      TranscriptEntry::Draft(MessageBuffer::default()),
      TranscriptEntry::Message(Message::agent(vec![AssistantContent::text(
        "foo",
      )])),
    ] {
      case(&[boundary, entry.clone()], None);
    }
  }

  #[test]
  fn unsupported_content() {
    let content = [
      AssistantContent::Image(::rig::message::Image::default()),
      AssistantContent::ToolCall(::rig::message::ToolCall::from_wire(
        "foo",
        ToolFunction {
          arguments: serde_json::json!({"bar": "baz"}),
          name: ToolName::new("command").unwrap(),
        },
      )),
    ];

    let expected = [TranscriptBlock::Raw("command {\"bar\":\"baz\"}".into())];

    assert_eq!(
      TranscriptBlock::content(&content, &[]).collect::<Vec<_>>(),
      expected,
    );

    let mut buffer = MessageBuffer::default();

    buffer.apply(MessageUpdate::Complete(content.to_vec().into()));

    assert_eq!(
      TranscriptBlock::draft(&buffer, &[]).collect::<Vec<_>>(),
      expected,
    );
  }
}
