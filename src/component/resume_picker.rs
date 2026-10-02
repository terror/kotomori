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
    let stack = StackComponent::default()
      .push(LineComponent::blank())
      .push_spaced(HeaderComponent::new(None))
      .push_spaced(LineComponent::from([
        Span::styled("Search previous sessions. Press ", Style::Muted),
        Span::styled("Enter", Style::Secondary),
        Span::styled(" to resume, ", Style::Muted),
        Span::styled("Esc", Style::Secondary),
        Span::styled(" to cancel.", Style::Muted),
      ]))
      .push_spaced(LineComponent::from([
        Span::styled("Search: ", Style::Muted),
        Span::raw(&self.picker.query),
        Span::styled(" ", Style::Selection),
      ]));

    let filtered = self.picker.filtered();

    if filtered.is_empty() {
      return stack
        .push(LineComponent::from([Span::styled(
          "No matching sessions.",
          Style::Muted,
        )]))
        .render(width);
    }

    filtered
      .into_iter()
      .enumerate()
      .fold(stack, |stack, (index, session)| {
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

        stack.push(LineComponent::from([
          Span::styled(marker, style),
          Span::styled(
            session.title.as_deref().unwrap_or("Untitled session"),
            style,
          ),
          Span::styled("  ", Style::Muted),
          Span::styled(session.detail(), Style::Muted),
        ]))
      })
      .render(width)
  }
}
