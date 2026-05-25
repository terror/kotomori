use super::*;

define_tool! {
  ReadFile {
    name: "read_file",
    description: "Read a UTF-8 text file.",
    arguments {
      required path: PathBuf => {"type": "string"},
    }
  }
}
