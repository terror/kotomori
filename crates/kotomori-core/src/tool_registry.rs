use super::*;

#[derive(Clone, Debug, Default)]
pub struct ToolRegistry {
  tools: BTreeMap<String, Arc<dyn Tool>>,
}

impl ToolRegistry {
  #[must_use]
  pub fn definitions(&self) -> Vec<ToolDefinition> {
    self.tools.values().map(|tool| tool.definition()).collect()
  }

  pub fn prepare(
    &self,
    call: ToolRequest,
  ) -> Result<(ToolInvocation, ToolCall)> {
    let name = call.function.name.as_str();

    let tool = self
      .tools
      .get(name)
      .with_context(|| format!("unknown tool `{name}`"))?
      .prepare(call.function.arguments.clone())
      .with_context(|| format!("failed to decode `{name}` arguments"))?;

    Ok((ToolInvocation { protocol: call }, tool))
  }

  pub fn register(&mut self, tool: Arc<dyn Tool>) -> Result {
    match self.tools.entry(tool.definition().name) {
      Entry::Occupied(entry) => {
        bail!("duplicate tool `{}`", entry.key());
      }
      Entry::Vacant(entry) => {
        entry.insert(tool);
        Ok(())
      }
    }
  }
}
