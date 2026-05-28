use super::*;

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct SessionFile {
  pub(crate) created_at: u64,
  pub(crate) cwd: PathBuf,
  pub(crate) entries: Vec<TranscriptEntry>,
  pub(crate) id: String,
  pub(crate) model: String,
  pub(crate) title: Option<String>,
  pub(crate) updated_at: u64,
  pub(crate) version: u32,
}
