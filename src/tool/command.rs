use super::*;

define_tool! {
  Command {
    name: "command",
    description: "Run a command and capture stdout, stderr, and exit status. Do not use this to list project files; use list_files instead.",
    arguments {
      required program: String => {"type": "string"},
      required arguments: Vec<String> => {
        "type": "array",
        "items": {"type": "string"}
      },
      optional cwd: Option<PathBuf> => {"type": ["string", "null"]},
    }
  }
}

impl Display for Command {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    if self.arguments.is_empty() {
      write!(f, "{}", self.program)
    } else {
      write!(f, "{} {}", self.program, self.arguments.join(" "))
    }
  }
}
