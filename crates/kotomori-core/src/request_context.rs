use super::*;

#[derive(Clone, Debug)]
pub struct RequestContext<'a> {
  pub generation: Cow<'a, GenerationOptions>,
  pub messages: Cow<'a, [Message]>,
  pub system: Option<Cow<'a, str>>,
  pub tools: Cow<'a, [ToolDefinition]>,
}

impl RequestContext<'_> {
  #[must_use]
  pub fn request<'a>(&'a self, model: &'a Model) -> Request<'a> {
    Request {
      generation: &self.generation,
      messages: &self.messages,
      model,
      system: self.system.as_deref(),
      tools: &self.tools,
    }
  }
}

impl<'a> From<Request<'a>> for RequestContext<'a> {
  fn from(request: Request<'a>) -> Self {
    Self {
      generation: Cow::Borrowed(request.generation),
      messages: Cow::Borrowed(request.messages),
      system: request.system.map(Cow::Borrowed),
      tools: Cow::Borrowed(request.tools),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn context_can_override_generation_without_changing_defaults() {
    let generation = GenerationOptions {
      max_output_tokens: NonZeroU64::new(1),
      ..Default::default()
    };

    let model = Model::default();

    let mut context = RequestContext::from(Request {
      generation: &generation,
      messages: &[],
      model: &model,
      system: None,
      tools: &[],
    });

    assert_eq!(context.request(&model).generation, &generation);

    context.generation.to_mut().max_output_tokens = NonZeroU64::new(2);

    assert_eq!(
      context.request(&model).generation,
      &GenerationOptions {
        max_output_tokens: NonZeroU64::new(2),
        ..Default::default()
      },
    );

    assert_eq!(
      generation,
      GenerationOptions {
        max_output_tokens: NonZeroU64::new(1),
        ..Default::default()
      },
    );
  }
}
