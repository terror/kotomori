use super::*;

#[derive(
  Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize,
)]
pub struct Usage {
  pub cache_creation_input_tokens: Option<u64>,
  pub cached_input_tokens: Option<u64>,
  pub input_tokens: Option<u64>,
  pub output_tokens: Option<u64>,
  pub reasoning_tokens: Option<u64>,
  pub tool_use_prompt_tokens: Option<u64>,
  pub total_tokens: Option<u64>,
}

impl AddAssign for Usage {
  fn add_assign(&mut self, other: Self) {
    for (total, tokens) in [
      (
        &mut self.cache_creation_input_tokens,
        other.cache_creation_input_tokens,
      ),
      (&mut self.cached_input_tokens, other.cached_input_tokens),
      (&mut self.input_tokens, other.input_tokens),
      (&mut self.output_tokens, other.output_tokens),
      (&mut self.reasoning_tokens, other.reasoning_tokens),
      (
        &mut self.tool_use_prompt_tokens,
        other.tool_use_prompt_tokens,
      ),
      (&mut self.total_tokens, other.total_tokens),
    ] {
      if let Some(tokens) = tokens {
        *total = Some(total.unwrap_or_default().saturating_add(tokens));
      }
    }
  }
}
