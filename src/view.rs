use super::*;

#[derive(Debug)]
pub(crate) struct View<'a> {
  state: &'a State,
}

impl<'a> View<'a> {
  pub(crate) fn new(state: &'a State) -> Self {
    Self { state }
  }

  pub(crate) fn render(&self, frame: &mut Frame) {
    let area = frame.area();

    let command_height = self.state.command_height();

    let transcript_height = self.state.transcript_height(area.width).min(
      area
        .height
        .saturating_sub(6u16.saturating_add(command_height)),
    );

    let [
      _,
      header_area,
      _,
      hint_area,
      _,
      transcript_area,
      composer_area,
      command_area,
      _,
    ] = Layout::default()
      .direction(Direction::Vertical)
      .constraints([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(transcript_height),
        Constraint::Length(1),
        Constraint::Length(command_height),
        Constraint::Min(0),
      ])
      .areas(area);

    frame.render_widget(Header, header_area);
    frame.render_widget(Hint, hint_area);
    frame.render_widget(Transcript::new(self.state), transcript_area);
    frame.render_widget(Composer::new(self.state), composer_area);
    frame.render_widget(Commands::new(self.state), command_area);

    let input_len =
      u16::try_from(self.state.input_text().len()).unwrap_or(u16::MAX);

    frame.set_cursor_position((
      composer_area.x.saturating_add(input_len).saturating_add(4),
      composer_area.y,
    ));
  }
}
