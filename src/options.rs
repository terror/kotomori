use super::*;

#[derive(Args, Debug)]
pub(crate) struct Options {
  #[arg(long, default_value = "fake:local")]
  pub(crate) model: Model,
  #[arg(short, long)]
  pub(crate) prompt: Option<String>,
}
