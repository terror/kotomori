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
        content_block: anthropic::ContentBlock::ToolUse { id, input, name },
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
        ToolCallUpdate {
          index,
          kind: ToolCallUpdateKind::Arguments(input),
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
