use super::*;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
pub(crate) struct ToolInvocation {
  pub(crate) id: String,
  pub(crate) kind: ToolInvocationKind,
}

impl ToolInvocation {
  pub(crate) fn completed_tense(&self) -> String {
    self.title(ToolActionTense::Completed)
  }

  pub(crate) fn failed_tense(&self) -> String {
    self.title(ToolActionTense::Failed)
  }

  pub(crate) fn from_raw<T>(call: RawToolCall) -> Result<ToolInvocationKind>
  where
    T: ToolSpec,
  {
    Ok(
      serde_json::from_value::<T>(call.arguments)
        .with_context(|| format!("failed to decode `{}` arguments", call.name))?
        .into(),
    )
  }

  pub(crate) fn progressive_tense(&self) -> String {
    self.title(ToolActionTense::Progressive)
  }

  fn title(&self, tense: ToolActionTense) -> String {
    format!("{} {}", self.kind.action(tense), self.kind.subject())
  }
}

impl Display for ToolInvocation {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    write!(f, "{}", self.kind.display())
  }
}

#[cfg(test)]
mod tests {
  use {super::*, serde_json::json};

  fn invocation(name: &str, arguments: Value) -> ToolInvocation {
    ToolRegistry::default()
      .invocation(RawToolCall {
        arguments,
        id: "foo".into(),
        name: name.into(),
      })
      .unwrap()
  }

  #[test]
  fn parses_command_tool_call() {
    let invocation =
      invocation("command", json!({"command": "bar baz", "cwd": null}));

    assert_eq!(
      invocation,
      ToolInvocation {
        id: "foo".into(),
        kind: ToolInvocationKind::Command(CommandTool {
          command: "bar baz".into(),
          cwd: None,
        }),
      },
    );
  }

  #[test]
  fn serializes_tool_arguments() {
    let invocation = ToolInvocation {
      id: "foo".into(),
      kind: ToolInvocationKind::Command(CommandTool {
        command: "baz bar".into(),
        cwd: None,
      }),
    };

    assert_eq!(invocation.kind.arguments(), json!({"command": "baz bar"}),);
  }

  #[test]
  fn unknown_tool_errors() {
    let result = ToolRegistry::default().invocation(RawToolCall {
      arguments: json!({}),
      id: "foo".into(),
      name: "bar".into(),
    });

    assert_eq!(result.unwrap_err().to_string(), "unknown tool `bar`",);
  }
}
