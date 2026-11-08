use super::*;

#[derive(Clone, Debug, Default)]
pub struct ToolRegistry {
  tools: BTreeMap<ToolName, (ToolDefinition, Arc<dyn Tool>)>,
}

impl ToolRegistry {
  #[must_use]
  pub fn definitions(&self) -> Vec<ToolDefinition> {
    self
      .tools
      .values()
      .map(|(definition, _)| definition.clone())
      .collect()
  }

  pub fn prepare(&self, call: ToolRequest) -> Result<(ToolRequest, ToolCall)> {
    let name = &call.function.name;

    let tool = self
      .tools
      .get(name)
      .with_context(|| format!("unknown tool `{name}`"))?
      .1
      .prepare(call.function.arguments.clone())
      .with_context(|| format!("failed to decode `{name}` arguments"))?;

    Ok((call, tool))
  }

  pub fn register(&mut self, tool: Arc<dyn Tool>) -> Result {
    let definition = tool.definition()?;

    match self.tools.entry(definition.name.clone()) {
      Entry::Occupied(entry) => {
        bail!("duplicate tool `{}`", entry.key());
      }
      Entry::Vacant(entry) => {
        entry.insert((definition, tool));
        Ok(())
      }
    }
  }
}
