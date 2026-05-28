use super::*;

#[derive(Debug)]
pub(crate) enum ResumePickerAction {
  Cancel,
  Resume(PathBuf),
}
