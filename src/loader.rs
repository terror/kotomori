use super::*;

#[derive(Debug, Clone)]
pub(crate) struct Loader {
  cwd: PathBuf,
}

impl Loader {
  const AGENTS: &'static str = "AGENTS.md";

  fn agent_paths(&self, root: &Path) -> Vec<PathBuf> {
    let mut paths = self
      .cwd
      .ancestors()
      .take_while(|ancestor| ancestor.starts_with(root))
      .map(|ancestor| ancestor.join(Self::AGENTS))
      .filter(|path| path.is_file())
      .collect::<Vec<_>>();

    paths.reverse();

    paths
  }

  fn agents(&self) -> Result<String> {
    let root = self.repository_root();

    self
      .agent_paths(&root)
      .into_iter()
      .map(|path| {
        Ok(format!(
          "{}:\n{}",
          path.display(),
          fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?
            .trim_end()
        ))
      })
      .collect::<Result<Vec<_>>>()
      .map(|agents| agents.join("\n\n"))
  }

  pub(crate) fn load(&self) -> Result<String> {
    self.agents()
  }

  pub(crate) fn new() -> Result<Self> {
    Ok(Self {
      cwd: env::current_dir().context("failed to get current directory")?,
    })
  }

  fn repository_root(&self) -> PathBuf {
    self
      .cwd
      .ancestors()
      .find(|ancestor| ancestor.join(".git").exists())
      .unwrap_or(&self.cwd)
      .to_path_buf()
  }

  #[cfg(test)]
  pub(crate) fn with_cwd(cwd: impl Into<PathBuf>) -> Self {
    Self { cwd: cwd.into() }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn loads_agents_from_root_to_cwd() {
    let directory = tempfile::tempdir().unwrap();

    let child = directory.path().join("foo").join("bar");

    fs::create_dir(directory.path().join(".git")).unwrap();
    fs::create_dir_all(&child).unwrap();
    fs::write(directory.path().join(Loader::AGENTS), "foo\n").unwrap();
    fs::write(child.join(Loader::AGENTS), "bar\n").unwrap();

    assert_eq!(
      Loader::with_cwd(child).load().unwrap(),
      format!(
        "{}:\nfoo\n\n{}:\nbar",
        directory.path().join(Loader::AGENTS).display(),
        directory
          .path()
          .join("foo")
          .join("bar")
          .join(Loader::AGENTS)
          .display()
      ),
    );
  }

  #[test]
  fn returns_empty_without_agents() {
    let directory = tempfile::tempdir().unwrap();

    fs::create_dir(directory.path().join(".git")).unwrap();

    assert_eq!(Loader::with_cwd(directory.path()).load().unwrap(), "");
  }
}
