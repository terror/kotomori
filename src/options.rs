use super::*;

#[derive(Args, Clone, Debug)]
pub(crate) struct Options {
  #[arg(long, default_value_t = Options::default_model(), global = true)]
  pub(crate) model: Model,
  #[arg(short, long, global = true)]
  pub(crate) prompt: Option<String>,
  #[arg(long, global = true)]
  pub(crate) yolo: bool,
}

impl Options {
  const DEFAULT_MODEL: &'static str = "mock:local";

  fn default_model() -> Model {
    Self::DEFAULT_MODEL
      .parse()
      .expect("default model should be valid")
  }

  pub(crate) fn is_default_model(&self) -> bool {
    self.model == Self::default_model()
  }
}
