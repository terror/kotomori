use super::*;

#[async_trait]
pub(crate) trait ToolSpec:
  DeserializeOwned + Into<ToolInvocationKind> + JsonSchema + Send + Sync
{
  /// Provider-facing description advertised with the tool schema.
  ///
  /// This text should explain when the model should call the tool and any
  /// constraints that are important before arguments are generated.
  const DESCRIPTION: &'static str;

  /// Stable provider-facing tool name.
  ///
  /// The name is used to advertise the tool to providers, decode raw tool calls
  /// back into a typed invocation, and serialize tool use messages back into
  /// conversation history.
  const NAME: &'static str;

  /// Verb used in transcript status lines for the requested tense.
  ///
  /// The returned text is combined with [`Self::subject`] to render pending,
  /// successful, and failed tool call status lines.
  fn action(tense: ToolActionTense) -> &'static str;

  /// Additional key-value metadata rendered below the transcript status line.
  ///
  /// Tools should return only compact details that help users understand what
  /// was invoked without expanding the full argument payload.
  fn details(&self) -> Vec<(&'static str, String)> {
    Vec::new()
  }

  /// Short approval prompt description for this concrete invocation.
  ///
  /// This should read naturally after "Approve", and should include the
  /// important target or command for tools that need user approval.
  fn display(&self) -> String;

  /// Executes the invocation and returns the provider-visible result.
  ///
  /// The caller supplies the executor so all tools share the same process,
  /// filesystem, timeout, and output-limit behavior.
  async fn execute(&self, executor: &Executor) -> ToolResult;

  /// Whether this concrete invocation requires approval before execution.
  ///
  /// Implementations can inspect their arguments, so approval policy can depend
  /// on the specific command or flags instead of only the tool type.
  fn requires_approval(&self) -> bool {
    false
  }

  /// Subject rendered after [`Self::action`] in transcript status lines.
  ///
  /// This text should be concise and focused on the object being acted on, such
  /// as a path, command, or search query.
  fn subject(&self) -> String;
}
