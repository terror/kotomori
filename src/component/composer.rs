use super::*;

#[derive(Debug)]
pub(crate) struct ComposerComponent<'a> {
  pub(super) composer: &'a Composer,
  pub(super) queued_inputs: &'a VecDeque<String>,
}

impl Component for ComposerComponent<'_> {
  fn render(&self, width: u16) -> Vec<LineComponent> {
    let mut lines = Vec::new();

    if !self.queued_inputs.is_empty() {
      lines.push(LineComponent::from([Span::styled("Queued", Style::Muted)]));

      for input in self.queued_inputs {
        lines
          .extend(GutteredLinesComponent::raw(input.split('\n')).render(width));
      }

      lines.push(LineComponent::blank());
    }

    let cursor = self.composer.cursor();

    let selected = self.composer.selected_command_index();

    lines.extend(
      GutteredLinesComponent::new(
        self.composer.lines().iter().enumerate().map(|(row, line)| {
          if cursor.0 != row {
            return LineComponent::raw(line);
          }

          let mut chars = line.chars();

          let before = chars.by_ref().take(cursor.1).collect::<String>();
          let under_cursor = chars.next().unwrap_or(' ');
          let after = chars.collect::<String>();

          LineComponent::from([
            Span::raw(before),
            Span::styled(under_cursor.to_string(), Style::Selection),
            Span::raw(after),
          ])
        }),
      )
      .render(width),
    );

    if selected.is_some() {
      lines.push(LineComponent::blank());
    }

    lines.extend(self.composer.commands().enumerate().map(
      |(index, command)| {
        let input_style = match selected {
          Some(selected) if selected == index => Style::Accent,
          _ => Style::Secondary,
        };

        LineComponent::from([
          Span::styled(command.input(), input_style),
          Span::styled("  ", Style::Muted),
          Span::styled(command.description(), Style::Muted),
        ])
      },
    ));

    lines
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn active_run_renders_queued_inputs() {
    let queued_inputs = VecDeque::from([
      "first follow-up".to_string(),
      "second\nfollow-up".to_string(),
    ]);

    let component = ComposerComponent {
      composer: &Composer::new("follow up", Vec::new()),
      queued_inputs: &queued_inputs,
    };

    assert_eq!(
      component.render(80),
      [
        LineComponent::from([Span::styled("Queued", Style::Muted)]),
        LineComponent::from([
          Span::styled("│ ", Style::Accent),
          Span::raw("first follow-up"),
        ]),
        LineComponent::from([
          Span::styled("│ ", Style::Accent),
          Span::raw("second"),
        ]),
        LineComponent::from([
          Span::styled("│ ", Style::Accent),
          Span::raw("follow-up"),
        ]),
        LineComponent::blank(),
        LineComponent::from([
          Span::styled("│ ", Style::Accent),
          Span::raw("follow up"),
          Span::styled(" ", Style::Selection),
        ]),
      ]
    );
  }

  #[test]
  fn command_rendering() {
    let component = ComposerComponent {
      composer: &Composer::new("/", Vec::new()),
      queued_inputs: &VecDeque::new(),
    };

    assert_eq!(
      component.render(80),
      [
        LineComponent::from([
          Span::styled("│ ", Style::Accent),
          Span::raw("/"),
          Span::styled(" ", Style::Selection),
        ]),
        LineComponent::blank(),
        LineComponent::from([
          Span::styled("/clear", Style::Accent),
          Span::styled("  ", Style::Muted),
          Span::styled("Clear the transcript", Style::Muted),
        ]),
        LineComponent::from([
          Span::styled("/quit", Style::Secondary),
          Span::styled("  ", Style::Muted),
          Span::styled("Quit kotomori", Style::Muted),
        ]),
      ]
    );
  }
}
