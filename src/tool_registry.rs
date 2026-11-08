use super::*;

#[derive(Clone, Debug, Default)]
pub(crate) struct ToolRegistry {
  tools: BTreeMap<String, Arc<dyn Tool>>,
}

impl ToolRegistry {
  pub(crate) fn definitions(&self) -> Vec<ToolDefinition> {
    self.tools.values().map(|tool| tool.definition()).collect()
  }

  pub(crate) fn prepare(
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

  pub(crate) fn register(&mut self, tool: Arc<dyn Tool>) -> Result {
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
