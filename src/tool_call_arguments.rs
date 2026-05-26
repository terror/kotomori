use super::*;

#[derive(Default)]
pub(crate) enum ToolCallArguments {
  #[default]
  Empty,
  Fragments(String),
  Value(Value),
}

impl ToolCallArguments {
  pub(crate) fn argument_fragment(self, argument_fragment: &str) -> Self {
    match self {
      Self::Empty | Self::Value(_) => Self::Fragments(argument_fragment.into()),
      Self::Fragments(mut argument_fragments) => {
        argument_fragments.push_str(argument_fragment);
        Self::Fragments(argument_fragments)
      }
    }
  }

  pub(crate) fn finish(self) -> Result<Value> {
    match self {
      Self::Empty => Ok(json!({})),
      Self::Fragments(arguments) => Ok(serde_json::from_str(&arguments)?),
      Self::Value(arguments) => Ok(arguments),
    }
  }
}
