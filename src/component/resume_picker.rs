use super::*;

#[derive(Debug)]
pub(crate) struct ResumePickerComponent<'a> {
  pub(crate) height: usize,
  pub(crate) picker: &'a ResumePicker,
}

impl Component for ResumePickerComponent<'_> {
  fn render(&self, width: u16) -> Vec<LineComponent> {
    let filtered = self.picker.filtered();

    let header = StackComponent::default()
      .push(LineComponent::blank())
      .push_spaced(HeaderComponent {
        first_draw_duration: None,
      })
      .push_spaced(LineComponent::from([
        Span::styled("Search previous sessions. Press ", Style::Muted),
        Span::styled("Enter", Style::Secondary),
        Span::styled(" to resume, ", Style::Muted),
        Span::styled("Esc", Style::Secondary),
        Span::styled(" to cancel.", Style::Muted),
      ]))
      .push_spaced(TextFieldComponent {
        label: Span::styled("Search: ", Style::Muted),
        value: &self.picker.query,
      });

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

          stack.push(ClipComponent {
            component: LineComponent::from([
              Span::styled(marker, style),
              Span::styled(
                session.title.as_deref().unwrap_or("Untitled session"),
                style,
              ),
              Span::styled("  ", Style::Muted),
              Span::styled(session.detail(), Style::Muted),
            ]),
            height: 1,
          })
        },
      )
    };

    ScrollComponent {
      component: sessions,
      focus: self.picker.selected,
      header: Some(Box::new(header)),
      height: self.height,
      offset: &self.picker.offset,
    }
    .render(width)
  }
}
