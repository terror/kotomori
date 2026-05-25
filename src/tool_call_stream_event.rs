use super::*;

pub(crate) trait ToolCallStreamEvent {
  type Index: Ord;

  fn tool_call_fragment(self) -> Option<ToolCallFragment<Self::Index>>;
}

impl ToolCallStreamEvent
  for openai::types::chat::ChatCompletionMessageToolCallChunk
{
  type Index = u32;

  fn tool_call_fragment(self) -> Option<ToolCallFragment<Self::Index>> {
    let (name, argument_fragment) = self
      .function
      .map_or((None, None), |function| (function.name, function.arguments));

    Some(ToolCallFragment::Update {
      argument_fragment,
      arguments: None,
      id: self.id,
      index: self.index,
      name,
    })
  }
}

impl ToolCallStreamEvent for anthropic::types::MessageStreamEvent {
  type Index = usize;

  fn tool_call_fragment(self) -> Option<ToolCallFragment<Self::Index>> {
    match self {
      Self::ContentBlockStart {
        content_block:
          anthropic::types::ContentBlock::ToolUse { id, input, name },
        index,
      } => Some(ToolCallFragment::Update {
        argument_fragment: None,
        arguments: Some(input),
        id: Some(id),
        index,
        name: Some(name),
      }),
      Self::ContentBlockDelta {
        delta:
          anthropic::types::ContentBlockDelta::InputJsonDelta { partial_json },
        index,
      } => Some(ToolCallFragment::Update {
        argument_fragment: Some(partial_json),
        arguments: None,
        id: None,
        index,
        name: None,
      }),
      Self::ContentBlockStop { index } => {
        Some(ToolCallFragment::Finish { index })
      }
      _ => None,
    }
  }
}
