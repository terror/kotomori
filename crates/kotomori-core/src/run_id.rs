use super::*;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(transparent)]
pub struct RunId(pub u64);

impl RunId {
  pub(crate) fn next() -> Self {
    static NEXT: AtomicU64 = AtomicU64::new(0);

    Self(
      NEXT
        .try_update(Ordering::Relaxed, Ordering::Relaxed, |id| {
          id.checked_add(1)
        })
        .expect("run ID overflow"),
    )
  }
}
