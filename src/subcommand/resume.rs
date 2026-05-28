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

  options.model = session.model().parse().with_context(|| {
    format!("failed to parse session model {}", session.model())
  })?;

  App::with_state(&options, State::with_session(&options, session)?)?
    .run()
    .await
}
