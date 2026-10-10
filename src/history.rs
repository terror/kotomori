use super::*;

#[derive(Debug)]
pub(crate) struct History {
  entries: Vec<String>,
  navigation: Option<HistoryNavigation>,
}

impl History {
  pub(crate) fn clear(&mut self) {
    self.entries.clear();
    self.reset_navigation();
  }

  pub(crate) fn new(entries: Vec<String>) -> Self {
    Self {
      entries,
      navigation: None,
    }
  }

  pub(crate) fn remember(&mut self, input: &str) {
    self.entries.push(input.into());
    self.reset_navigation();
  }

  pub(crate) fn reset_navigation(&mut self) {
    self.navigation = None;
  }

  pub(crate) fn select_next(&mut self) -> Option<String> {
    let navigation = self.navigation.take()?;

    let index = navigation.index.saturating_add(1);

    if let Some(input) = self.entries.get(index).cloned() {
      self.navigation = Some(HistoryNavigation {
        index,
        ..navigation
      });

      Some(input)
    } else {
      Some(navigation.draft)
    }
  }

  pub(crate) fn select_previous(&mut self, draft: String) -> Option<String> {
    let index = if let Some(navigation) = &self.navigation {
      navigation.index.checked_sub(1)
    } else {
      self.entries.len().checked_sub(1)
    }?;

    if let Some(navigation) = &mut self.navigation {
      navigation.index = index;
    } else {
      self.navigation = Some(HistoryNavigation { draft, index });
    }

    Some(self.entries[index].clone())
  }
}
