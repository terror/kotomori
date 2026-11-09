use super::*;

#[derive(Clone, Debug)]
pub struct Request<'a> {
  pub generation: &'a GenerationOptions,
  pub messages: &'a [Message],
  pub model: &'a Model,
  pub system: Option<&'a str>,
  pub tools: &'a [ToolDefinition],
}

impl Request<'_> {
  pub fn last_user_text(&self) -> Option<&str> {
    self.messages.iter().rev().find_map(Message::user_content)
  }

  pub(super) fn validate(&self) -> Result {
    self.generation.validate()?;

    match &self.generation.tool_choice {
      Some(ToolChoice::Required) => {
        ensure!(!self.tools.is_empty(), "tool choice requires tools");
      }
      Some(ToolChoice::Specific(names)) => {
        ensure!(!names.is_empty(), "tool choice requires at least one tool");

        for name in names {
          ensure!(
            self.tools.iter().any(|tool| tool.name == *name),
            "tool choice refers to unavailable tool `{name}`"
          );
        }
      }
      _ => {}
    }

    ensure!(
      Conversation::pending_calls(self.messages)?.is_empty(),
      "incomplete tool exchange in context"
    );

    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use {super::*, serde_json::json};

  #[test]
  fn generation_options_are_validated() {
    #[track_caller]
    fn case(
      generation: &GenerationOptions,
      tools: &[ToolDefinition],
      error: Option<&str>,
    ) {
      assert_eq!(
        Request {
          generation,
          messages: &[],
          model: &Model::default(),
          system: None,
          tools,
        }
        .validate()
        .err()
        .map(|error| error.to_string())
        .as_deref(),
        error,
      );
    }

    case(&GenerationOptions::default(), &[], None);

    for temperature in [-1.0, f64::NAN, f64::INFINITY] {
      case(
        &GenerationOptions {
          temperature: Some(temperature),
          ..Default::default()
        },
        &[],
        Some("temperature must be finite and nonnegative"),
      );
    }

    case(
      &GenerationOptions {
        output_schema: Some(json!("foo")),
        ..Default::default()
      },
      &[],
      Some("output schema must be a JSON object or boolean"),
    );

    for (choice, error) in [
      (ToolChoice::Auto, None),
      (ToolChoice::None, None),
      (ToolChoice::Required, Some("tool choice requires tools")),
      (
        ToolChoice::Specific(Vec::new()),
        Some("tool choice requires at least one tool"),
      ),
      (
        ToolChoice::Specific(vec![ToolName::new("foo").unwrap()]),
        Some("tool choice refers to unavailable tool `foo`"),
      ),
    ] {
      case(
        &GenerationOptions {
          tool_choice: Some(choice),
          ..Default::default()
        },
        &[],
        error,
      );
    }

    case(
      &GenerationOptions {
        max_output_tokens: NonZeroU64::new(1),
        output_schema: Some(json!(false)),
        temperature: Some(0.0),
        tool_choice: Some(ToolChoice::Specific(vec![
          ToolName::new("foo").unwrap(),
        ])),
      },
      &[ToolDefinition {
        description: "bar".into(),
        name: ToolName::new("foo").unwrap(),
        parameters: json!({}),
      }],
      None,
    );
  }
}
