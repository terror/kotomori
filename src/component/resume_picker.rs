use super::*;

#[derive(Debug)]
pub(crate) struct ResumePickerComponent<'a> {
  picker: &'a ResumePicker,
}

impl<'a> ResumePickerComponent<'a> {
  pub(crate) fn new(picker: &'a ResumePicker) -> Self {
    Self { picker }
  }
}

impl Component for ResumePickerComponent<'_> {
  fn render(&self, width: u16) -> Vec<LineComponent> {
    let mut lines = once(LineComponent::blank())
      .chain(HeaderComponent::new(None).render(width))
      .chain(once(LineComponent::blank()))
      .chain(once(LineComponent::from([
        Span::styled("Search previous sessions. Press ", Style::Muted),
        Span::styled("Enter", Style::Secondary),
        Span::styled(" to resume, ", Style::Muted),
        Span::styled("Esc", Style::Secondary),
        Span::styled(" to cancel.", Style::Muted),
      ])))
      .chain(once(LineComponent::blank()))
      .chain(once(LineComponent::from([
        Span::styled("Search: ", Style::Muted),
        Span::raw(&self.picker.query),
        Span::styled(" ", Style::Selection),
      ])))
      .chain(once(LineComponent::blank()))
      .collect::<Vec<_>>();

    let filtered = self.picker.filtered();

    if filtered.is_empty() {
      lines.push(LineComponent::from([Span::styled(
        "No matching sessions.",
        Style::Muted,
      )]));

      return lines;
    }

    for (index, session) in filtered.into_iter().enumerate() {
      let style = if index == self.picker.selected {
        Style::Accent
      } else {
        Style::Secondary
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
        Span::styled("  ", Style::Muted),
        Span::styled(session.detail(), Style::Muted),
      ]));
    }

    lines
  }
}
