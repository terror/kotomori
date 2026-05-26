use super::*;

#[derive(Default)]
pub(crate) enum ToolCallArguments {
  Deltas(String),
  #[default]
  Empty,
}

impl ToolCallArguments {
  pub(crate) fn argument_delta(&mut self, argument_delta: &str) -> &mut Self {
    match self {
      Self::Empty => *self = Self::Deltas(argument_delta.into()),
      Self::Deltas(argument_deltas) => {
        argument_deltas.push_str(argument_delta);
      }
    }

    self
  }

  pub(crate) fn finish(self) -> Result<Value> {
    match self {
      Self::Empty => Ok(json!({})),
      Self::Deltas(arguments) => Ok(serde_json::from_str(&arguments)?),
    }
  }
}
