use super::*;

#[derive(Args, Debug)]
pub(crate) struct Options {
  #[arg(long, default_value = "mock:local")]
  pub(crate) model: Model,
  #[arg(short, long)]
  pub(crate) prompt: Option<String>,
  #[arg(long)]
  pub(crate) yolo: bool,
}
