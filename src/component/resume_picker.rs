use super::*;

#[derive(Debug)]
pub(crate) struct ResumePickerComponent {
  stack: StackComponent<'static>,
}

impl ResumePickerComponent {
  pub(crate) fn layout(
    picker: &mut ResumePicker,
    dimensions: Dimensions,
  ) -> Self {
    let (header, gap) = if dimensions.height == 0 {
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
              label: Span::styled("Search: ", Style::Muted),
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

    let (rows, focus) = if filtered.is_empty() {
      (
        LineComponent::from([Span::styled(
          "No matching sessions.",
          Style::Muted,
        )])
        .render(dimensions.width),
        0..0,
      )
    } else {
      filtered.into_iter().enumerate().fold(
        (Vec::new(), 0..0),
        |(mut rows, focus), (index, session)| {
          let (marker, style) = if index == picker.selected {
            ("> ", Style::Accent)
          } else {
            ("  ", Style::Secondary)
          };

          let start = rows.len();

          rows.extend(
            LineComponent::from([
              Span::styled(marker, style),
              Span::styled(
                session.title.as_deref().unwrap_or("Untitled session"),
                style,
              ),
              Span::styled("  ", Style::Muted),
              Span::styled(session.detail(), Style::Muted),
            ])
            .render(dimensions.width),
          );

          let focus = if index == picker.selected {
            start..rows.len()
          } else {
            focus
          };

          (rows, focus)
        },
      )
    };

    picker.scroll.update(focus, height, rows.len());

    Self {
      stack: StackComponent::default()
        .gap(gap)
        .push(LinesComponent { lines: header })
        .push(ScrollComponent {
          component: LinesComponent { lines: rows },
          height,
          offset: picker.scroll.offset,
        }),
    }
  }
}

impl Component for ResumePickerComponent {
  fn render(&self, width: u16) -> Vec<LineComponent> {
    self.stack.render(width)
  }
}
