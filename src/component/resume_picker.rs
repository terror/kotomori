use super::*;

#[derive(Debug)]
pub(crate) struct ResumePickerComponent<'a> {
  height: usize,
  picker: &'a ResumePicker,
}

impl<'a> ResumePickerComponent<'a> {
  pub(crate) fn new(picker: &'a ResumePicker, height: usize) -> Self {
    Self { height, picker }
  }
}

impl Component for ResumePickerComponent<'_> {
  fn render(&self, width: u16) -> Vec<LineComponent> {
    let filtered = self.picker.filtered();

    let sessions = if filtered.is_empty() {
      StackComponent::default().push(LineComponent::from([Span::styled(
        "No matching sessions.",
        Style::Muted,
      )]))
    } else {
      filtered.into_iter().enumerate().fold(
        StackComponent::default(),
        |stack, (index, session)| {
          let (marker, style) = if index == self.picker.selected {
            ("> ", Style::Accent)
          } else {
            ("  ", Style::Secondary)
          };

          stack.push(ClipComponent::new(
            LineComponent::from([
              Span::styled(marker, style),
              Span::styled(
                session.title.as_deref().unwrap_or("Untitled session"),
                style,
              ),
              Span::styled("  ", Style::Muted),
              Span::styled(session.detail(), Style::Muted),
            ]),
            1,
          ))
        },
      )
    };

    ScrollComponent::new(
      sessions,
      self.height,
      self.picker.selected,
      &self.picker.offset,
    )
    .header(
      StackComponent::default()
        .push(LineComponent::blank())
        .push_spaced(HeaderComponent::new(None))
        .push_spaced(LineComponent::from([
          Span::styled("Search previous sessions. Press ", Style::Muted),
          Span::styled("Enter", Style::Secondary),
          Span::styled(" to resume, ", Style::Muted),
          Span::styled("Esc", Style::Secondary),
          Span::styled(" to cancel.", Style::Muted),
        ]))
        .push_spaced(TextFieldComponent::new(
          Span::styled("Search: ", Style::Muted),
          &self.picker.query,
        )),
    )
    .render(width)
  }
}
