use super::*;

#[derive(Args, Clone, Debug, Eq, PartialEq)]
pub(crate) struct Options {
  #[arg(long, global = true)]
  pub(crate) model: Option<Model>,
  #[arg(short, long, global = true)]
  pub(crate) prompt: Option<String>,
  #[arg(long, global = true)]
  pub(crate) yolo: bool,
}
