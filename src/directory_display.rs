use super::*;

pub(crate) struct DirectoryDisplay<'a> {
  home: Option<PathBuf>,
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
    Self::with_home(path, env::home_dir())
  }

  fn with_home(path: &'a Path, home: Option<PathBuf>) -> Self {
    Self { home, path }
  }
}

impl Display for DirectoryDisplay<'_> {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    let path = self.path.clean();

    let Some(home) = self.home.as_ref().map(|home| home.clean()) else {
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
  fn displays_paths() {
    #[track_caller]
    fn case(path: &str, expected: &str) {
      assert_eq!(DirectoryDisplay::new(Path::new(path)).to_string(), expected);
    }

    case("foo/bar/..", "foo");
    case(
      "/foo/./bar",
      &Path::new("/foo/bar").clean().display().to_string(),
    );
    case(
      "foo/./bar",
      &Path::new("foo").join("bar").display().to_string(),
    );
  }

  #[test]
  fn displays_paths_with_home() {
    #[track_caller]
    fn case(path: &str, expected: &str) {
      assert_eq!(
        DirectoryDisplay::with_home(Path::new(path), Some("/foo".into()))
          .to_string(),
        expected,
      );
    }

    case(
      "/foo/bar/../baz",
      &Path::new("~").join("baz").display().to_string(),
    );
    case(
      "/foo/bar",
      &Path::new("~").join("bar").display().to_string(),
    );
    case("/foo", "~");
    case("/bar", &Path::new("/bar").clean().display().to_string());
  }
}
