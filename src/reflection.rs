use super::*;

#[derive(Debug, Default)]
pub(crate) struct Reflection {
  content: String,
  id: Option<String>,
}

impl Reflection {
  pub(crate) fn delta(
    &mut self,
    id: Option<String>,
    delta: impl Into<String>,
  ) -> Option<String> {
    let delta = delta.into();

    if delta.is_empty() {
      return None;
    }

    if self.id != id {
      self.content.clear();
      self.id = id;
    }

    self.content.push_str(&delta);

    Some(delta)
  }

  pub(crate) fn reasoning(&mut self, reasoning: Reasoning) -> Option<String> {
    let (text, id) = (reasoning.display_text(), reasoning.id);

    if text.is_empty() {
      return None;
    }

    let delta = if self.id == id && text.starts_with(&self.content) {
      text[self.content.len()..].to_string()
    } else {
      text.clone()
    };

    (self.content, self.id) = (text, id);

    (!delta.is_empty()).then_some(delta.clone())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn delta_appends_when_id_matches() {
    let mut reflection = Reflection::default();

    assert_eq!(
      reflection.delta(Some("foo".into()), "bar"),
      Some("bar".into())
    );

    assert_eq!(
      reflection.delta(Some("foo".into()), "baz"),
      Some("baz".into())
    );

    assert_eq!(
      reflection.reasoning(Reasoning::new("barbazqux").with_id("foo".into())),
      Some("qux".into())
    );
  }

  #[test]
  fn delta_ignores_empty_text() {
    let mut reflection = Reflection::default();

    assert_eq!(reflection.delta(None, ""), None);
  }

  #[test]
  fn delta_resets_content_when_id_changes() {
    let mut reflection = Reflection::default();

    assert_eq!(
      reflection.delta(Some("foo".into()), "bar"),
      Some("bar".into())
    );

    assert_eq!(
      reflection.delta(Some("qux".into()), "baz"),
      Some("baz".into())
    );

    assert_eq!(
      reflection.reasoning(Reasoning::new("bazquux").with_id("qux".into())),
      Some("quux".into())
    );
  }

  #[test]
  fn reasoning_deduplicates_matching_content() {
    let mut reflection = Reflection::default();

    assert_eq!(reflection.delta(None, "foo"), Some("foo".into()));

    assert_eq!(reflection.reasoning(Reasoning::new("foo")), None);

    assert_eq!(
      reflection.reasoning(Reasoning::new("foobar")),
      Some("bar".into())
    );
  }

  #[test]
  fn reasoning_ignores_empty_text() {
    let mut reflection = Reflection::default();

    assert_eq!(reflection.reasoning(Reasoning::new("")), None);
  }

  #[test]
  fn reasoning_returns_full_text_when_content_does_not_match() {
    let mut reflection = Reflection::default();

    assert_eq!(reflection.delta(None, "foo"), Some("foo".into()));

    assert_eq!(
      reflection.reasoning(Reasoning::new("bar")),
      Some("bar".into())
    );
  }

  #[test]
  fn reasoning_returns_full_text_when_id_changes() {
    let mut reflection = Reflection::default();

    assert_eq!(
      reflection.delta(Some("foo".into()), "bar"),
      Some("bar".into())
    );

    assert_eq!(
      reflection.reasoning(Reasoning::new("barbaz").with_id("qux".into())),
      Some("barbaz".into())
    );
  }
}
