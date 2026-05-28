use super::*;

#[derive(Args, Clone, Debug)]
pub(crate) struct Options {
  #[arg(long, default_value = "mock:local", global = true)]
  pub(crate) model: Model,
  #[arg(short, long, global = true)]
  pub(crate) prompt: Option<String>,
  #[arg(long, global = true)]
  pub(crate) yolo: bool,
}
