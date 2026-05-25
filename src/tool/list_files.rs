use super::*;

define_tool! {
  ListFiles {
    name: "list_files",
    description: "List project files while respecting .gitignore and other standard ignore rules.",
    arguments {
      optional cwd: Option<PathBuf> => {"type": ["string", "null"]},
    }
    invocation |tool| ToolInvocationKind::ListFiles(CommandInvocation {
      arguments: vec!["--files".into()],
      cwd: tool.cwd,
      program: "rg".into(),
    }),
  }
}
