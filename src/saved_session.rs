use super::*;

#[derive(Debug, PartialEq)]
pub(crate) struct SavedSession {
  pub(crate) id: i64,
  pub(crate) title: Option<String>,
  pub(crate) updated_at: u64,
}
