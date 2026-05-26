use super::*;

#[derive(Default)]
pub(crate) enum ToolCallArguments {
  Deltas(String),
  #[default]
  Empty,
  Value(Value),
}

impl ToolCallArguments {
  pub(crate) fn argument_delta(self, argument_delta: &str) -> Result<Self> {
    Ok(match self {
      Self::Empty => Self::Deltas(argument_delta.into()),
      Self::Deltas(mut argument_deltas) => {
        argument_deltas.push_str(argument_delta);
        Self::Deltas(argument_deltas)
      }
      Self::Value(_) => {
        bail!("received tool call argument delta after complete arguments")
      }
    })
  }

  pub(crate) fn finish(self) -> Result<Value> {
    match self {
      Self::Empty => Ok(json!({})),
      Self::Deltas(arguments) => Ok(serde_json::from_str(&arguments)?),
      Self::Value(arguments) => Ok(arguments),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn argument_delta_after_arguments_errors() {
    let result = ToolCallArguments::Value(json!({"foo": "bar"}))
      .argument_delta(r#"{"baz": "qux"}"#);

    assert_eq!(
      result.err().unwrap().to_string(),
      "received tool call argument delta after complete arguments",
    );
  }
}
