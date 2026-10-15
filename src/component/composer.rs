use super::*;

#[derive(Debug)]
pub(crate) struct ComposerComponent<'a> {
  pub(crate) composer: &'a Composer,
}

impl Component for ComposerComponent<'_> {
  fn render(&self, width: u16) -> Vec<LineComponent> {
    let cursor = self.composer.cursor();

    let input = LinesComponent {
      lines: self
        .composer
        .lines()
        .iter()
        .enumerate()
        .map(|(row, line)| {
          if cursor.0 != row {
            return LineComponent::raw(line);
          }

          let mut chars = line.chars();

          let (before, under_cursor, after) = (
            chars.by_ref().take(cursor.1).collect::<String>(),
            chars.next().unwrap_or(' '),
            chars.collect::<String>(),
          );

          LineComponent::from([
            Span::raw(before),
            Span::styled(under_cursor.to_string(), Style::Selection),
            Span::raw(after),
          ])
        })
        .collect(),
    };

    StackComponent::default()
      .gap(1)
      .push(input.gutter(Span::styled("│ ", Style::Accent)))
      .push(LinesComponent {
        lines: self
          .composer
          .commands()
          .enumerate()
          .map(|(index, command)| {
            LineComponent::from([
              Span::styled(
                command.input(),
                match self.composer.selected_command_index() {
                  Some(selected) if selected == index => Style::Accent,
                  _ => Style::Secondary,
                },
              ),
              Span::styled("  ", Style::Muted),
              Span::styled(command.description(), Style::Muted),
            ])
          })
          .collect(),
      })
      .render(width)
  }
}
