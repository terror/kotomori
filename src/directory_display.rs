use super::*;

pub(crate) struct DirectoryDisplay<'a> {
  path: &'a Path,
}

impl<'a> DirectoryDisplay<'a> {
  fn format_relative(relative: &Path) -> String {
    if relative.as_os_str().is_empty() {
      "~".to_owned()
    } else {
      Path::new("~").join(relative).display().to_string()
    }
  }

  pub(crate) fn new(path: &'a Path) -> Self {
    Self { path }
  }

  #[cfg(test)]
  fn with_home(path: &Path, home: &Path) -> String {
    let path = path.lexiclean();

    match path.strip_prefix(home.lexiclean()).ok() {
      Some(relative) => Self::format_relative(relative),
      None => path.display().to_string(),
    }
  }
}

impl Display for DirectoryDisplay<'_> {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    let path = self.path.lexiclean();

    let Some(home) = home_dir().map(|home| home.lexiclean()) else {
      return write!(f, "{}", path.display());
    };

    match path.strip_prefix(home).ok() {
      Some(relative) => write!(f, "{}", Self::format_relative(relative)),
      None => write!(f, "{}", path.display()),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn cleans_before_matching_home() {
    assert_eq!(
      DirectoryDisplay::with_home(
        Path::new("/foo/bar/../baz"),
        Path::new("/foo"),
      ),
      Path::new("~").join("baz").display().to_string(),
    );
  }

  #[test]
  fn cleans_parent_components() {
    assert_eq!(
      DirectoryDisplay::new(Path::new("foo/bar/..")).to_string(),
      "foo"
    );
  }

  #[test]
  fn displays_absolute_path() {
    assert_eq!(
      DirectoryDisplay::new(Path::new("/foo/./bar")).to_string(),
      Path::new("/foo/bar").lexiclean().display().to_string(),
    );
  }

  #[test]
  fn displays_child_of_home_with_tilde() {
    assert_eq!(
      DirectoryDisplay::with_home(Path::new("/foo/bar"), Path::new("/foo")),
      Path::new("~").join("bar").display().to_string(),
    );
  }

  #[test]
  fn displays_home_as_tilde() {
    assert_eq!(
      DirectoryDisplay::with_home(Path::new("/foo"), Path::new("/foo")),
      "~",
    );
  }

  #[test]
  fn displays_path_outside_home() {
    assert_eq!(
      DirectoryDisplay::with_home(Path::new("/bar"), Path::new("/foo")),
      Path::new("/bar").lexiclean().display().to_string(),
    );
  }

  #[test]
  fn displays_relative_path() {
    assert_eq!(
      DirectoryDisplay::new(Path::new("foo/./bar")).to_string(),
      Path::new("foo").join("bar").display().to_string(),
    );
  }
}
