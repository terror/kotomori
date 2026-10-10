use super::*;

#[derive(Clone, Debug)]
pub(crate) struct Loader {
  pub(crate) cwd: PathBuf,
}

impl Loader {
  const AGENTS: &'static str = "AGENTS.md";

  pub(crate) fn load(&self) -> Result<String> {
    let root = self
      .cwd
      .ancestors()
      .find(|ancestor| ancestor.join(".git").exists())
      .unwrap_or(&self.cwd);

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
      .map(|path| {
        let contents = fs::read_to_string(&path)
          .with_context(|| format!("failed to read {}", path.display()))?;

        Ok(format!("{}:\n{}", path.display(), contents.trim_end()))
      })
      .collect::<Result<Vec<_>>>()
      .map(|agents| agents.join("\n\n"))
  }

  pub(crate) fn new(cwd: impl Into<PathBuf>) -> Self {
    Self { cwd: cwd.into() }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn loads_agents_from_root_to_cwd() {
    let directory = temptree! {
      ".git": {},
      "AGENTS.md": "foo\n",
      foo: {
        bar: {
          "AGENTS.md": "bar\n",
        },
      },
    };

    let root = directory.path();
    let child = root.join("foo").join("bar");

    let root_agents = root.join(Loader::AGENTS);
    let child_agents = child.join(Loader::AGENTS);

    assert_eq!(
      Loader::new(child).load().unwrap(),
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

    assert_eq!(Loader::new(directory.path()).load().unwrap(), "");
  }
}
