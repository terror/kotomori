use super::*;

#[derive(Clone, Debug)]
pub(crate) struct SessionSummary {
  pub(crate) cwd: PathBuf,
  pub(crate) model: String,
  pub(crate) path: PathBuf,
  pub(crate) search: String,
  pub(crate) title: String,
  pub(crate) updated_at: u64,
}

impl SessionSummary {
  fn age(&self) -> String {
    let Ok(now) = SessionStore::now() else {
      return "unknown age".into();
    };

    let seconds = now.saturating_sub(self.updated_at);

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
      SessionStore::display_directory(&self.cwd),
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

  pub(crate) fn new(path: PathBuf, file: SessionFile) -> Self {
    let title = file
      .title
      .or_else(|| Session::title(&file.entries))
      .unwrap_or_else(|| "Untitled session".into());

    let directory = SessionStore::display_directory(&file.cwd);

    let search = format!("{} {} {} {}", title, file.model, directory, file.id)
      .chars()
      .flat_map(char::to_lowercase)
      .collect();

    Self {
      cwd: file.cwd,
      model: file.model,
      path,
      search,
      title,
      updated_at: file.updated_at,
    }
  }

  pub(crate) fn path(&self) -> &Path {
    &self.path
  }

  pub(crate) fn title(&self) -> &str {
    &self.title
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn summary_matches_all_query_terms() {
    let summary = SessionSummary {
      cwd: "baz".into(),
      model: "mock:local".into(),
      path: "qux".into(),
      search: "foo bar baz".into(),
      title: "foo".into(),
      updated_at: SessionStore::now().unwrap(),
    };

    assert!(summary.matches("foo baz"));

    assert!(!summary.matches("foo qux"));
  }
}
