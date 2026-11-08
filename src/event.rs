use super::*;

#[derive(Debug, PartialEq)]
pub(crate) enum Event {
  Action(Action),
  Agent(RunEvent),
  ClipboardCopied(Result<(), String>),
  Error(String),
  RunSpawned(RunId),
  SessionForked(Result<Session, String>),
  SessionSaved(Result<SavedSession, String>),
  Tick(Duration),
}
