use super::*;

#[derive(Debug)]
pub(crate) struct ResumePickerComponent<'a> {
  pub(crate) picker: &'a mut ResumePicker,
}

impl ResumePickerComponent<'_> {
  pub(crate) fn layout(
    &mut self,
    dimensions: Dimensions,
  ) -> Vec<LineComponent> {
    let picker = &mut *self.picker;

    let (mut header, gap) = if dimensions.height == 0 {
      (Vec::new(), 0)
    } else {
      let mut lines = StackComponent::default()
        .push(LineComponent::blank())
        .push(
          StackComponent::default()
            .gap(1)
            .push(HeaderComponent {
              first_draw_duration: None,
            })
            .push(LineComponent::from([
              Span::styled("Search previous sessions. Press ", Style::Muted),
              Span::styled("Enter", Style::Secondary),
              Span::styled(" to resume, ", Style::Muted),
              Span::styled("Esc", Style::Secondary),
              Span::styled(" to cancel.", Style::Muted),
            ]))
            .push(TextFieldComponent {
              label: LineComponent::styled("Search: ", Style::Muted),
              value: &picker.query,
            }),
        )
        .render(dimensions.width);

      let height = dimensions.height.saturating_sub(1).max(1);

      let gap = if lines.len() + 1 > height {
        lines.retain(|line| !line.is_blank());

        lines.drain(..lines.len().saturating_sub(height));

        0
      } else {
        1
      };

      (lines, gap)
    };

    let height = dimensions.height.saturating_sub(header.len() + gap);

    let filtered = picker.filtered();

    let selected = picker.selection.index(filtered.len());

    let items = if filtered.is_empty() {
      vec![LineComponent::styled("No matching sessions.", Style::Muted)]
    } else {
      filtered
        .into_iter()
        .enumerate()
        .map(|(index, session)| {
          let (marker, style) = if selected == Some(index) {
            ("> ", Style::Accent)
          } else {
            ("  ", Style::Secondary)
          };

          LineComponent::from([
            Span::styled(marker, style),
            Span::styled(
              session.title.as_deref().unwrap_or("Untitled session"),
              style,
            ),
            Span::styled("  ", Style::Muted),
            Span::styled(session.detail(), Style::Muted),
          ])
        })
        .collect()
    };

    let rows = ListComponent { items, selected }.layout(
      Dimensions {
        height,
        ..dimensions
      },
      &mut picker.scroll,
    );

    if !rows.is_empty() {
      header.resize_with(header.len() + gap, LineComponent::blank);
      header.extend(rows);
    }

    header
  }
}
