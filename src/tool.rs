use super::*;

mod command;

pub(crate) use command::CommandTool;

macro_rules! define_tools {
  ($( $variant:ident($tool:ty), )*) => {
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(tag = "name", content = "arguments", rename_all = "snake_case")]
    pub(crate) enum ToolInvocationKind {
      $(
        $variant($tool),
      )*
    }

    impl ToolInvocationKind {
      pub(crate) fn action(&self, tense: ToolActionTense) -> &'static str {
        match self {
          $(Self::$variant(_) => <$tool>::action(tense),)*
        }
      }

      pub(crate) fn approval(&self) -> ApprovalPolicy {
        match self {
          $(Self::$variant(tool) => tool.approval(),)*
        }
      }

      pub(crate) fn arguments(&self) -> Value {
        match self {
          $(Self::$variant(tool) => serde_json::to_value(tool),)*
        }
        .expect("failed to serialize tool arguments")
      }

      pub(crate) fn decode(call: RawToolCall) -> Result<ToolInvocation> {
        let kind = match call.name.as_str() {
          $(
            <$tool>::NAME => Self::$variant(
              serde_json::from_value(call.arguments).with_context(|| {
                format!("failed to decode `{}` arguments", call.name)
              })?,
            ),
          )*
          _ => bail!("unknown tool `{}`", call.name),
        };

        Ok(ToolInvocation { id: call.id, kind })
      }

      pub(crate) fn definitions() -> Vec<ToolDefinition> {
        vec![$(ToolDefinition {
          description: <$tool>::DESCRIPTION.into(),
          name: <$tool>::NAME.into(),
          parameters: serde_json::to_value(<$tool>::json_schema(
            &mut schemars::SchemaGenerator::default(),
          ))
          .expect("failed to serialize tool schema"),
        }),*]
      }

      pub(crate) fn details(&self) -> Vec<(&'static str, String)> {
        match self {
          $(Self::$variant(tool) => tool.details(),)*
        }
      }

      pub(crate) async fn execute(&self, context: &ToolContext) -> ToolResult {
        match self {
          $(Self::$variant(tool) => tool.execute(context).await,)*
        }
      }

      pub(crate) fn name(&self) -> &'static str {
        match self {
          $(Self::$variant(_) => <$tool>::NAME,)*
        }
      }
    }

    impl Display for ToolInvocationKind {
      fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
          $(Self::$variant(tool) => Display::fmt(tool, f),)*
        }
      }
    }

    $(
      impl From<$tool> for ToolInvocationKind {
        fn from(tool: $tool) -> Self {
          Self::$variant(tool)
        }
      }
    )*
  };
}

define_tools! {
  Command(CommandTool),
}
