use super::*;

pub trait SessionStore {
  type Key: ?Sized;

  fn load(&self, key: &Self::Key) -> Result<Option<SessionSnapshot>>;

  fn save(&self, key: &Self::Key, snapshot: &SessionSnapshot) -> Result;
}
