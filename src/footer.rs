use super::*;

#[derive(Debug)]
pub(crate) struct Footer {
  text: String,
}

impl Footer {
  pub(crate) fn new(model: &Model) -> Result<Self> {
    let directory =
      env::current_dir().context("failed to read current directory")?;

    let directory = match env::var_os("HOME").map(PathBuf::from) {
      Some(home) => match directory.strip_prefix(home) {
        Ok(relative) if relative.as_os_str().is_empty() => "~".to_string(),
        Ok(relative) => format!("~/{}", relative.display()),
        Err(_) => directory.display().to_string(),
      },
      None => directory.display().to_string(),
    };

    Ok(Self {
      text: format!("{} · {} · {directory}", model.provider(), model.name()),
    })
  }

  #[cfg(test)]
  pub(crate) fn raw(text: impl Into<String>) -> Self {
    Self { text: text.into() }
  }
}

impl Component for Footer {
  fn render(&self, _width: u16) -> Vec<Line> {
    vec![vec![Span::styled(&self.text, Style::DarkGray)].into()]
  }
}
