use super::*;

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct GenerationOptions {
  pub max_output_tokens: Option<NonZeroU64>,
  pub output_schema: Option<Value>,
  pub temperature: Option<f64>,
  pub tool_choice: Option<ToolChoice>,
}

impl GenerationOptions {
  pub fn validate(&self) -> Result {
    if let Some(temperature) = self.temperature {
      ensure!(
        temperature.is_finite() && temperature >= 0.0,
        "temperature must be finite and nonnegative"
      );
    }

    if let Some(schema) = &self.output_schema {
      ensure!(
        schema.is_object() || schema.is_boolean(),
        "output schema must be a JSON object or boolean"
      );
    }

    Ok(())
  }
}
