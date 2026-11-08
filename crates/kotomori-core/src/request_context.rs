use super::*;

#[derive(Clone, Debug)]
pub struct RequestContext<'a> {
  pub messages: Cow<'a, [Message]>,
  pub system: Option<Cow<'a, str>>,
  pub tools: Cow<'a, [ToolDefinition]>,
}

impl RequestContext<'_> {
  #[must_use]
  pub fn request<'a>(&'a self, model: &'a Model) -> Request<'a> {
    Request {
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
      messages: Cow::Borrowed(request.messages),
      system: request.system.map(Cow::Borrowed),
      tools: Cow::Borrowed(request.tools),
    }
  }
}
