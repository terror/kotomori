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
  fn format() {
    #[track_caller]
    fn case(duration: Duration, expected: &str) {
      assert_eq!(duration.format(), expected);
    }

    case(Duration::from_secs(0), "0s");
    case(Duration::from_secs(59), "59s");
    case(Duration::from_mins(1), "1m 0s");
    case(Duration::from_secs(111), "1m 51s");
    case(Duration::from_mins(2), "2m 0s");
  }
}
