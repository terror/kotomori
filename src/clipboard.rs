use super::*;

#[derive(Default)]
pub(crate) struct Clipboard {
  inner: Option<arboard::Clipboard>,
}

impl Clipboard {
  pub(crate) fn copy(&mut self, text: String) -> Result {
    let clipboard = match &mut self.inner {
      Some(clipboard) => clipboard,
      clipboard => clipboard.insert(arboard::Clipboard::new()?),
    };

    clipboard.set_text(text)?;

    Ok(())
  }
}

impl Debug for Clipboard {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    f.debug_struct("Clipboard").finish_non_exhaustive()
  }
}
