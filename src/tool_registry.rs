use super::*;

#[derive(Clone, Debug)]
pub(crate) struct ToolRegistry {
  tools: Arc<[Tool]>,
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
      kind: tool.invocation(call)?,
    })
  }

  pub(crate) fn new(tools: Vec<Tool>) -> Self {
    Self {
      tools: tools.into(),
    }
  }

  pub(crate) fn tools(&self) -> impl Iterator<Item = &Tool> {
    self.tools.iter()
  }
}

impl Default for ToolRegistry {
  fn default() -> Self {
    static DEFAULT: LazyLock<ToolRegistry> = LazyLock::new(|| {
      ToolRegistry::new(vec![
        Tool::new::<ApplyPatchTool>(),
        Tool::new::<CommandTool>(),
        Tool::new::<ListFilesTool>(),
        Tool::new::<ReadFileTool>(),
        Tool::new::<SearchFilesTool>(),
        Tool::new::<WriteFileTool>(),
      ])
    });

    DEFAULT.clone()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn empty_registry_does_not_decode_default_tools() {
    let result = ToolRegistry::new(Vec::new()).invocation(RawToolCall::new(
      "foo",
      "command",
      json!({"program": "bar", "arguments": []}),
    ));

    assert_eq!(result.unwrap_err().to_string(), "unknown tool `command`");
  }
}
