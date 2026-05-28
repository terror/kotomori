use super::*;

pub(crate) async fn run(mut options: Options) -> Result {
  let sessions = SessionStore::list()?;

  if sessions.is_empty() {
    println!("No saved sessions.");
    return Ok(());
  }

  let Some(path) = ResumePicker::new(sessions).run()? else {
    return Ok(());
  };

  let session = SessionStore::load(&path)?;

  options.model = session.file.model.parse().with_context(|| {
    format!("failed to parse session model {}", session.file.model)
  })?;

  App::with_state(&options, State::with_session(&options, session)?)?
    .run()
    .await
}
