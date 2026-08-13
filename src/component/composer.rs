use super::*;

#[derive(Debug)]
pub(crate) struct ComposerComponent<'a> {
  pub(super) agent_active: bool,
  pub(super) composer: &'a Composer,
  pub(super) queued_input_count: usize,
}

impl Component for ComposerComponent<'_> {
  fn render(&self, width: u16) -> Vec<LineComponent> {
    let cursor = self.composer.cursor();

    let selected = self.composer.selected_command_index();

    let mut lines = GutteredLinesComponent::new(
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
    .render(width);

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

    if self.agent_active {
      let queued = match self.queued_input_count {
        0 => String::new(),
        count => format!(" · {count} queued"),
      };

      lines.push(LineComponent::from([
        Span::styled("Enter", Style::Secondary),
        Span::styled(" queue · ", Style::Muted),
        Span::styled("Alt-Enter", Style::Secondary),
        Span::styled(format!(" interrupt now{queued}"), Style::Muted),
      ]));
    }

    lines
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn active_run_renders_steering_controls_and_queue_count() {
    let component = ComposerComponent {
      agent_active: true,
      composer: &Composer::new("follow up", Vec::new()),
      queued_input_count: 2,
    };

    assert_eq!(
      component.render(80).last().unwrap(),
      &LineComponent::from([
        Span::styled("Enter", Style::Secondary),
        Span::styled(" queue · ", Style::Muted),
        Span::styled("Alt-Enter", Style::Secondary),
        Span::styled(" interrupt now · 2 queued", Style::Muted),
      ])
    );
  }

  #[test]
  fn command_rendering() {
    let component = ComposerComponent {
      agent_active: false,
      composer: &Composer::new("/", Vec::new()),
      queued_input_count: 0,
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
