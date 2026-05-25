use super::*;

define_tool! {
  SearchFiles {
    name: "search_files",
    description: "Search files with ripgrep.",
    arguments {
      required arguments: Vec<String> => {
        "type": "array",
        "items": {"type": "string"}
      },
      optional cwd: Option<PathBuf> => {"type": ["string", "null"]},
    }
    invocation |tool| ToolInvocationKind::SearchFiles(CommandInvocation {
      arguments: tool.arguments,
      cwd: tool.cwd,
      program: "rg".into(),
    }),
  }
}
