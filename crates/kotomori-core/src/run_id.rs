use super::*;

#[derive(
  Clone,
  Copy,
  Debug,
  Deserialize,
  Eq,
  Hash,
  Ord,
  PartialEq,
  PartialOrd,
  Serialize,
)]
pub struct RunId(u64);

impl RunId {
  pub(super) fn new() -> Self {
    static NEXT: AtomicU64 = AtomicU64::new(1);

    Self(NEXT.fetch_add(1, Ordering::Relaxed))
  }
}
