use super::*;

#[derive(Args, Debug)]
pub(crate) struct Options {
  #[arg(long, default_value = "local")]
  pub(crate) model: String,
  #[arg(short, long)]
  pub(crate) prompt: Option<String>,
}
