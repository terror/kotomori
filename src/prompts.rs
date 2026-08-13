pub(crate) fn compaction() -> &'static str {
  include_str!("../prompts/compaction.md").trim()
}

pub(crate) fn system() -> &'static str {
  include_str!("../prompts/system.md").trim()
}
