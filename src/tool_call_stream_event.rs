use super::*;

pub(crate) trait ToolCallStreamEvent {
  type Index: Ord;

  fn tool_call_updates(self) -> Vec<ToolCallUpdate<Self::Index>>;
}

impl ToolCallStreamEvent for anthropic::MessageStreamEvent {
  type Index = usize;

  fn tool_call_updates(self) -> Vec<ToolCallUpdate<Self::Index>> {
    match self {
      Self::ContentBlockStart {
        content_block: anthropic::ContentBlock::ToolUse { id, name, .. },
        index,
      } => vec![
        ToolCallUpdate {
          index,
          kind: ToolCallUpdateKind::Id(id),
        },
        ToolCallUpdate {
          index,
          kind: ToolCallUpdateKind::Name(name),
        },
      ],
      Self::ContentBlockDelta {
        delta: anthropic::ContentBlockDelta::InputJsonDelta { partial_json },
        index,
      } => vec![ToolCallUpdate {
        index,
        kind: ToolCallUpdateKind::ArgumentDelta(partial_json),
      }],
      Self::ContentBlockStop { index } => vec![ToolCallUpdate {
        index,
        kind: ToolCallUpdateKind::Finish,
      }],
      _ => Vec::new(),
    }
  }
}

impl ToolCallStreamEvent for openai::ChatCompletionMessageToolCallChunk {
  type Index = u32;

  fn tool_call_updates(self) -> Vec<ToolCallUpdate<Self::Index>> {
    let mut updates = Vec::new();

    if let Some(id) = self.id {
      updates.push(ToolCallUpdate {
        index: self.index,
        kind: ToolCallUpdateKind::Id(id),
      });
    }

    if let Some(function) = self.function {
      if let Some(name) = function.name {
        updates.push(ToolCallUpdate {
          index: self.index,
          kind: ToolCallUpdateKind::Name(name),
        });
      }

      if let Some(argument_delta) = function.arguments {
        updates.push(ToolCallUpdate {
          index: self.index,
          kind: ToolCallUpdateKind::ArgumentDelta(argument_delta),
        });
      }
    }

    updates
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn anthropic_tool_call_arguments_are_streamed_as_deltas() {
    let mut stream = ToolCallStream::<usize>::default();

    assert_eq!(
      stream
        .push_event(anthropic::MessageStreamEvent::ContentBlockStart {
          content_block: anthropic::ContentBlock::ToolUse {
            id: "foo".into(),
            input: json!({}),
            name: "read_file".into(),
          },
          index: 0,
        })
        .unwrap(),
      Vec::new(),
    );

    assert_eq!(
      stream
        .push_event(anthropic::MessageStreamEvent::ContentBlockDelta {
          delta: anthropic::ContentBlockDelta::InputJsonDelta {
            partial_json: r#"{"path":"#.into(),
          },
          index: 0,
        })
        .unwrap(),
      Vec::new(),
    );

    assert_eq!(
      stream
        .push_event(anthropic::MessageStreamEvent::ContentBlockDelta {
          delta: anthropic::ContentBlockDelta::InputJsonDelta {
            partial_json: r#""foo"}"#.into(),
          },
          index: 0,
        })
        .unwrap(),
      Vec::new(),
    );

    assert_eq!(
      stream
        .push_event(anthropic::MessageStreamEvent::ContentBlockStop {
          index: 0,
        })
        .unwrap(),
      vec![RawToolCall::new("foo", "read_file", json!({"path": "foo"}))],
    );
  }
}
