use super::*;

mod command;

pub(crate) use command::CommandTool;

macro_rules! define_tools {
  ($( $variant:ident($tool:ty), )*) => {
    #[derive(Clone, Debug, Eq, PartialEq)]
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

      #[cfg(test)]
      pub(crate) fn arguments(&self) -> Value {
        match self {
          $(Self::$variant(tool) => serde_json::to_value(tool),)*
        }
        .expect("failed to serialize tool arguments")
      }

      pub(crate) fn decode(call: ::rig::message::ToolCall) -> Result<ToolInvocation> {
        let kind = match call.function.name.as_str() {
          $(
            <$tool>::NAME => Self::$variant(
              serde_json::from_value(call.function.arguments.clone()).with_context(|| {
                format!("failed to decode `{}` arguments", call.function.name)
              })?,
            ),
          )*
          _ => bail!("unknown tool `{}`", call.function.name),
        };

        Ok(ToolInvocation { kind, protocol: call })
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

      #[cfg(test)]
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
