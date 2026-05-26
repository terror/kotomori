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
        ToolCallUpdate::Id { id, index },
        ToolCallUpdate::Name { index, name },
        ToolCallUpdate::Arguments {
          arguments: input,
          index,
        },
      ],
      Self::ContentBlockDelta {
        delta: anthropic::ContentBlockDelta::InputJsonDelta { partial_json },
        index,
      } => vec![ToolCallUpdate::ArgumentDelta {
        argument_delta: partial_json,
        index,
      }],
      Self::ContentBlockStop { index } => {
        vec![ToolCallUpdate::Finish { index }]
      }
      _ => Vec::new(),
    }
  }
}

impl ToolCallStreamEvent for openai::ChatCompletionMessageToolCallChunk {
  type Index = u32;

  fn tool_call_updates(self) -> Vec<ToolCallUpdate<Self::Index>> {
    let mut updates = Vec::new();

    if let Some(id) = self.id {
      updates.push(ToolCallUpdate::Id {
        id,
        index: self.index,
      });
    }

    if let Some(function) = self.function {
      if let Some(name) = function.name {
        updates.push(ToolCallUpdate::Name {
          index: self.index,
          name,
        });
      }

      if let Some(argument_delta) = function.arguments {
        updates.push(ToolCallUpdate::ArgumentDelta {
          argument_delta,
          index: self.index,
        });
      }
    }

    updates
  }
}
