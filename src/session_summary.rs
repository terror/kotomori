use super::*;

#[derive(Clone, Debug)]
pub(crate) struct SessionSummary {
  pub(crate) cwd: PathBuf,
  pub(crate) id: String,
  pub(crate) model: String,
  pub(crate) search: String,
  pub(crate) title: String,
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
      DirectoryDisplay::new(&self.cwd),
      self.age()
    )
  }

  pub(crate) fn matches(&self, query: &str) -> bool {
    query.split_whitespace().all(|term| {
      self.search.contains(
        &term
          .chars()
          .flat_map(char::to_lowercase)
          .collect::<String>(),
      )
    })
  }

  pub(crate) fn new(
    cwd: PathBuf,
    id: String,
    model: String,
    title: Option<String>,
    updated_at: u64,
  ) -> Self {
    let title = title.unwrap_or_else(|| "Untitled session".into());
    let directory = DirectoryDisplay::new(&cwd);

    let search = format!("{title} {model} {directory} {id}")
      .chars()
      .flat_map(char::to_lowercase)
      .collect();

    Self {
      cwd,
      id,
      model,
      search,
      title,
      updated_at,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn summary_matches_all_query_terms() {
    let summary = SessionSummary {
      cwd: "baz".into(),
      id: "qux".into(),
      model: "mock:local".into(),
      search: "foo bar baz".into(),
      title: "foo".into(),
      updated_at: SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs(),
    };

    assert!(summary.matches("foo baz"));

    assert!(!summary.matches("foo qux"));
  }
}
