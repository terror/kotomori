use super::*;

pub(crate) trait DurationExt {
  fn format(self) -> String;
}

impl DurationExt for Duration {
  fn format(self) -> String {
    let total_seconds = self.as_secs();

    let minutes = total_seconds / 60;
    let seconds = total_seconds % 60;

    if minutes == 0 {
      format!("{seconds}s")
    } else {
      format!("{minutes}m {seconds}s")
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn formats_exact_minute_boundaries() {
    assert_eq!(Duration::from_mins(1).format(), "1m 0s");
    assert_eq!(Duration::from_mins(2).format(), "2m 0s");
  }

  #[test]
  fn formats_minutes_and_seconds_when_at_least_one_minute() {
    assert_eq!(Duration::from_mins(1).format(), "1m 0s");
    assert_eq!(Duration::from_secs(111).format(), "1m 51s");
  }

  #[test]
  fn formats_seconds_only_when_under_one_minute() {
    assert_eq!(Duration::from_secs(0).format(), "0s");
    assert_eq!(Duration::from_secs(59).format(), "59s");
  }
}
