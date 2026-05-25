use super::*;

define_tool! {
  ApplyPatch {
    name: "apply_patch",
    description: "Apply a unified patch to the workspace.",
    arguments {
      required patch: String => {"type": "string"},
      optional cwd: Option<PathBuf> => {"type": ["string", "null"]},
    }
    invocation |tool| ToolInvocationKind::ApplyPatch {
      cwd: tool.cwd,
      patch: tool.patch,
    },
  }
}
