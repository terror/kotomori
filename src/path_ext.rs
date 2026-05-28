use super::*;

pub(crate) trait PathExt {
  fn directory_display(&self) -> DirectoryDisplay<'_>;
}

impl PathExt for Path {
  fn directory_display(&self) -> DirectoryDisplay<'_> {
    DirectoryDisplay::new(self)
  }
}
