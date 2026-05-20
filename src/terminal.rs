use super::*;

#[derive(Debug)]
pub(crate) struct Terminal {
  inner: DefaultTerminal,
}

impl Drop for Terminal {
  fn drop(&mut self) {
    if let Err(error) = ratatui::try_restore() {
      eprintln!("failed to restore terminal: {error}");
    }
  }
}

impl Terminal {
  pub(crate) fn draw(&mut self, f: impl FnOnce(&mut Frame)) -> Result {
    self.inner.draw(f)?;
    Ok(())
  }

  pub(crate) fn new() -> Result<Self> {
    Ok(Self {
      inner: ratatui::try_init().context("failed to initialize terminal")?,
    })
  }
}
