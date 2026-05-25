use super::*;

#[derive(Debug)]
pub(crate) struct View<'a> {
  state: &'a State,
}

impl<'a> View<'a> {
  pub(crate) fn new(state: &'a State) -> Self {
    Self { state }
  }
}

impl Component for View<'_> {
  fn render(&self, width: u16) -> Vec<Line> {
    once(Line::blank())
      .chain(Header.render(width))
      .chain(once(Line::blank()))
      .chain(Hint.render(width))
      .chain(once(Line::blank()))
      .chain(self.state.transcript().render(width))
      .chain(self.state.composer().render(width))
      .collect()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn composer_renders_while_agent_is_active() {
    let mut state = State::new(&Options {
      model: "fake:local".parse().unwrap(),
      prompt: Some("foo".into()),
    })
    .unwrap();

    state.handle_event(Event::Action(Action::Submit));

    assert!(state.transcript().is_agent_active());

    assert!(
      View::new(&state)
        .render(80)
        .iter()
        .any(|line| line.to_string().contains("fake · local ·"))
    );
  }
}
