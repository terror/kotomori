use super::*;

#[derive(Debug, Clone)]
pub(crate) struct Loader {
  cwd: PathBuf,
}

impl Loader {
  const AGENTS: &'static str = "AGENTS.md";

  fn agent_paths<'a>(
    &'a self,
    root: &'a Path,
  ) -> impl Iterator<Item = PathBuf> + 'a {
    let mut ancestors = self
      .cwd
      .ancestors()
      .take_while(|ancestor| ancestor.starts_with(root))
      .collect::<Vec<_>>();

    ancestors.reverse();

    ancestors
      .into_iter()
      .map(|directory| directory.join(Self::AGENTS))
      .filter(|path| path.is_file())
  }

  pub(crate) fn load(&self) -> Result<String> {
    let root = self.repository_root();

    self
      .agent_paths(&root)
      .map(|path| {
        let contents = fs::read_to_string(&path)
          .with_context(|| format!("failed to read {}", path.display()))?;

        Ok(format!("{}:\n{}", path.display(), contents.trim_end()))
      })
      .collect::<Result<Vec<_>>>()
      .map(|agents| agents.join("\n\n"))
  }

  pub(crate) fn cwd(&self) -> &Path {
    &self.cwd
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

    let root = directory.path();
    let child = root.join("foo").join("bar");

    let root_agents = root.join(Loader::AGENTS);
    let child_agents = child.join(Loader::AGENTS);

    fs::create_dir(root.join(".git")).unwrap();
    fs::create_dir_all(&child).unwrap();

    fs::write(&root_agents, "foo\n").unwrap();
    fs::write(&child_agents, "bar\n").unwrap();

    assert_eq!(
      Loader::with_cwd(child).load().unwrap(),
      format!(
        "{}:\nfoo\n\n{}:\nbar",
        root_agents.display(),
        child_agents.display(),
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
