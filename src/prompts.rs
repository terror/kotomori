use super::*;

macro_rules! prompts {
  ($($name:ident => $path:literal),+ $(,)?) => {
    $(
      pub(crate) static $name: LazyLock<String> = LazyLock::new(|| {
        include_str!(concat!("../prompts/", $path))
          .trim_end()
          .to_string()
      });
    )+
  };
}

prompts! {
  COMPACTION_PROMPT => "compaction.md",
  SYSTEM_PROMPT => "system.md",
}
