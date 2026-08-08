use super::*;

#[derive(Clone, Debug)]
pub(crate) struct ToolRegistry {
  pub(crate) tools: Arc<[Tool]>,
}

impl ToolRegistry {
  pub(crate) fn invocation(&self, call: RawToolCall) -> Result<ToolInvocation> {
    let name = call.name.clone();

    let tool = self
      .tools
      .iter()
      .find(|tool| tool.name == name)
      .with_context(|| format!("unknown tool `{name}`"))?;

    Ok(ToolInvocation {
      id: call.id.clone(),
      kind: (tool.invocation)(call)?,
    })
  }

  pub(crate) fn new(tools: Vec<Tool>) -> Self {
    Self {
      tools: tools.into(),
    }
  }
}

macro_rules! tool_registry_tools {
  ($( $variant:ident($tool:ty), )*) => {
    vec![
      $(
        Tool::new::<$tool>(),
      )*
    ]
  };
}

impl Default for ToolRegistry {
  fn default() -> Self {
    static DEFAULT: LazyLock<ToolRegistry> =
      LazyLock::new(|| ToolRegistry::new(define_tools!(tool_registry_tools)));

    DEFAULT.clone()
  }
}

#[cfg(test)]
mod tests {
  use {super::*, serde_json::json};

  #[test]
  fn default_registry_contains_tools() {
    assert_eq!(
      ToolRegistry::default()
        .tools
        .iter()
        .map(|tool| tool.name)
        .collect::<Vec<_>>(),
      ["command"],
    );
  }

  #[test]
  fn empty_registry_does_not_decode_default_tools() {
    let result = ToolRegistry::new(Vec::new()).invocation(RawToolCall {
      arguments: json!({"program": "bar", "arguments": []}),
      id: "foo".into(),
      name: "command".into(),
    });

    assert_eq!(result.unwrap_err().to_string(), "unknown tool `command`");
  }
}
