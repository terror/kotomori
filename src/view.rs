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
    let composer = if self.state.transcript().is_agent_active() {
      Vec::new()
    } else {
      self.state.composer().render(width)
    };

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
