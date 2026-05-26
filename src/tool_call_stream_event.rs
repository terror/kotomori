use super::*;

pub(crate) trait ToolCallStreamEvent {
  type Index: Ord;

  fn tool_call_fragments(self) -> Vec<ToolCallFragment<Self::Index>>;
}

impl ToolCallStreamEvent for anthropic::MessageStreamEvent {
  type Index = usize;

  fn tool_call_fragments(self) -> Vec<ToolCallFragment<Self::Index>> {
    match self {
      Self::ContentBlockStart {
        content_block: anthropic::ContentBlock::ToolUse { id, input, name },
        index,
      } => vec![
        ToolCallFragment::Id { id, index },
        ToolCallFragment::Name { index, name },
        ToolCallFragment::Arguments {
          arguments: input,
          index,
        },
      ],
      Self::ContentBlockDelta {
        delta: anthropic::ContentBlockDelta::InputJsonDelta { partial_json },
        index,
      } => vec![ToolCallFragment::ArgumentFragment {
        argument_fragment: partial_json,
        index,
      }],
      Self::ContentBlockStop { index } => {
        vec![ToolCallFragment::Finish { index }]
      }
      _ => Vec::new(),
    }
  }
}

impl ToolCallStreamEvent for openai::ChatCompletionMessageToolCallChunk {
  type Index = u32;

  fn tool_call_fragments(self) -> Vec<ToolCallFragment<Self::Index>> {
    let mut fragments = Vec::new();

    if let Some(id) = self.id {
      fragments.push(ToolCallFragment::Id {
        id,
        index: self.index,
      });
    }

    if let Some(function) = self.function {
      if let Some(name) = function.name {
        fragments.push(ToolCallFragment::Name {
          index: self.index,
          name,
        });
      }

      if let Some(argument_fragment) = function.arguments {
        fragments.push(ToolCallFragment::ArgumentFragment {
          argument_fragment,
          index: self.index,
        });
      }
    }

    fragments
  }
}
