use super::*;

#[derive(Debug)]
pub(crate) struct ComposerComponent<'a> {
  composer: &'a Composer,
}

impl<'a> ComposerComponent<'a> {
  pub(crate) fn new(composer: &'a Composer) -> Self {
    Self { composer }
  }
}

impl Component for ComposerComponent<'_> {
  fn render(&self, width: u16) -> Vec<LineComponent> {
    let cursor = self.composer.cursor();

    let selected = self.composer.selected_command_index();

    let mut lines = FramedLinesComponent::new(
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
  fn command_rendering() {
    let composer = Composer::new("/");

    let lines = ComposerComponent::new(&composer).render(80);

    assert_eq!(
      lines[1],
      LineComponent::from([
        Span::styled("/clear", Style::Accent),
        Span::styled("  ", Style::Muted),
        Span::styled("Clear the transcript", Style::Muted),
      ])
    );

    assert_eq!(
      lines[2],
      LineComponent::from([
        Span::styled("/quit", Style::Secondary),
        Span::styled("  ", Style::Muted),
        Span::styled("Quit kotomori", Style::Muted),
      ])
    );
  }
}
