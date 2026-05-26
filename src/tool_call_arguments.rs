use super::*;

#[derive(Default)]
pub(crate) enum ToolCallArguments {
  Deltas(String),
  #[default]
  Empty,
  Value(Value),
}

impl ToolCallArguments {
  pub(crate) fn argument_delta(self, argument_delta: &str) -> Self {
    match self {
      Self::Empty | Self::Value(_) => Self::Deltas(argument_delta.into()),
      Self::Deltas(mut argument_deltas) => {
        argument_deltas.push_str(argument_delta);
        Self::Deltas(argument_deltas)
      }
    }
  }

  pub(crate) fn finish(self) -> Result<Value> {
    match self {
      Self::Empty => Ok(json!({})),
      Self::Deltas(arguments) => Ok(serde_json::from_str(&arguments)?),
      Self::Value(arguments) => Ok(arguments),
    }
  }
}
