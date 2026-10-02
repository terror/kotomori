use super::*;

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct SessionSummary {
  pub(crate) directory: PathBuf,
  pub(crate) id: i64,
  pub(crate) model: Model,
  pub(crate) title: Option<String>,
  pub(crate) updated_at: u64,
}

impl SessionSummary {
  fn age(&self) -> String {
    let Ok(now) = SystemTime::now().duration_since(UNIX_EPOCH) else {
      return "unknown age".into();
    };

    let seconds = now.as_secs().saturating_sub(self.updated_at);

    match seconds {
      0..=59 => "now".into(),
      60..=3_599 => format!("{}m ago", seconds / 60),
      3_600..=86_399 => format!("{}h ago", seconds / 3_600),
      _ => format!("{}d ago", seconds / 86_400),
    }
  }

  pub(crate) fn detail(&self) -> String {
    format!(
      "{} · {} · {}",
      self.model,
      DirectoryDisplay::new(&self.directory),
      self.age()
    )
  }

  pub(crate) fn matches(&self, query: &str) -> bool {
    let search = format!(
      "{} {} {} {}",
      self.title.as_deref().unwrap_or("Untitled session"),
      self.model,
      DirectoryDisplay::new(&self.directory),
      self.id,
    )
    .chars()
    .flat_map(char::to_lowercase)
    .collect::<String>();

    query.split_whitespace().all(|term| {
      search.contains(
        &term
          .chars()
          .flat_map(char::to_lowercase)
          .collect::<String>(),
      )
    })
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn matches_queries() {
    #[track_caller]
    fn case(title: Option<&str>, query: &str, expected: bool) {
      let summary = SessionSummary {
        directory: "foo".into(),
        id: 42,
        model: "mock:bar".parse().unwrap(),
        title: title.map(Into::into),
        updated_at: 0,
      };

      assert_eq!(summary.matches(query), expected);
    }

    case(Some("baz"), "", true);
    case(Some("baz"), "  BAZ\tMOCK:BAR\nFOO 42  ", true);
    case(Some("baz"), "baz 43", false);
    case(Some("baz"), "qux", false);
    case(Some("baz"), "Untitled", false);
    case(None, "UNTITLED SESSION 42", true);
    case(None, "baz", false);
  }
}
