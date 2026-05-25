use super::*;

#[derive(Debug)]
pub(crate) struct View<'a> {
  footer: &'a str,
  state: &'a State,
}

impl<'a> View<'a> {
  pub(crate) fn new(state: &'a State, footer: &'a str) -> Self {
    Self { footer, state }
  }
}

impl Component for View<'_> {
  fn render(&self, width: u16) -> Vec<Line> {
    let composer = self
      .state
      .composer()
      .render_with_footer(width, Some(self.footer));

    once(Line::blank())
      .chain(Header.render(width))
      .chain(once(Line::blank()))
      .chain(Hint.render(width))
      .chain(once(Line::blank()))
      .chain(
        self
          .state
          .transcript()
          .render_active(self.state.active_frame(), width),
      )
      .chain(composer)
      .collect()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn composer_renders_while_agent_is_active() {
    let mut state = State::new("foo");

    state.handle_event(Event::Action(Action::Submit));

    assert!(state.transcript().is_agent_active());

    assert!(
      View::new(&state, "bar")
        .render(80)
        .contains(&vec![Span::styled("bar", Style::DarkGray)].into())
    );
  }
}
