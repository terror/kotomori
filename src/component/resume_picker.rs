use super::*;

#[derive(Debug)]
pub(crate) struct ResumePickerComponent<'a> {
  first_draw_duration: Option<Duration>,
  picker: &'a ResumePicker,
}

impl<'a> ResumePickerComponent<'a> {
  pub(crate) fn new(
    picker: &'a ResumePicker,
    first_draw_duration: Option<Duration>,
  ) -> Self {
    Self {
      first_draw_duration,
      picker,
    }
  }
}

impl Component for ResumePickerComponent<'_> {
  fn render(&self, width: u16) -> Vec<LineComponent> {
    let mut lines = once(LineComponent::blank())
      .chain(HeaderComponent::new(self.first_draw_duration).render(width))
      .chain(once(LineComponent::blank()))
      .chain(once(LineComponent::from([
        Span::styled("Search previous sessions. Press ", Style::DarkGray),
        Span::styled("Enter", Style::Gray),
        Span::styled(" to resume, ", Style::DarkGray),
        Span::styled("Esc", Style::Gray),
        Span::styled(" to cancel.", Style::DarkGray),
      ])))
      .chain(once(LineComponent::blank()))
      .chain(once(LineComponent::from([
        Span::styled("Search: ", Style::DarkGray),
        Span::raw(&self.picker.query),
        Span::styled(" ", Style::Reverse),
      ])))
      .chain(once(LineComponent::blank()))
      .collect::<Vec<_>>();

    let filtered = self.picker.filtered();

    if filtered.is_empty() {
      lines.push(LineComponent::from([Span::styled(
        "No matching sessions.",
        Style::DarkGray,
      )]));

      return lines;
    }

    for (index, session) in filtered.into_iter().enumerate() {
      let style = if index == self.picker.selected {
        Style::CyanBold
      } else {
        Style::Gray
      };

      let marker = if index == self.picker.selected {
        "> "
      } else {
        "  "
      };

      lines.push(LineComponent::from([
        Span::styled(marker, style),
        Span::styled(
          session.title.as_deref().unwrap_or("Untitled session"),
          style,
        ),
        Span::styled("  ", Style::DarkGray),
        Span::styled(session.detail(), Style::DarkGray),
      ]));
    }

    lines
  }
}
