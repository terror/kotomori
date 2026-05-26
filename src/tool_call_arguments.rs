use super::*;

#[derive(Default)]
pub(crate) enum ToolCallArguments {
  #[default]
  Empty,
  Fragments(String),
  Value(Value),
}

impl ToolCallArguments {
  pub(crate) fn argument_fragment(
    self,
    argument_fragment: Option<&str>,
  ) -> Self {
    let Some(argument_fragment) = argument_fragment else {
      return self;
    };

    match self {
      Self::Empty | Self::Value(_) => Self::Fragments(argument_fragment.into()),
      Self::Fragments(mut argument_fragments) => {
        argument_fragments.push_str(argument_fragment);
        Self::Fragments(argument_fragments)
      }
    }
  }

  pub(crate) fn arguments(self, arguments: Option<Value>) -> Self {
    match arguments {
      Some(arguments) => Self::Value(arguments),
      None => self,
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
