use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CommandInvocation {
  pub(crate) arguments: Vec<String>,
  pub(crate) cwd: Option<PathBuf>,
  pub(crate) program: String,
}

impl Display for CommandInvocation {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    if self.arguments.is_empty() {
      write!(f, "{}", self.program)
    } else {
      write!(f, "{} {}", self.program, self.arguments.join(" "))
    }
  }
}
