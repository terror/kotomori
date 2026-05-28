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
          Span::styled(under_cursor.to_string(), Style::Reverse),
          Span::raw(after),
        ])
      }),
    )
    .render(width);

    lines.extend(self.composer.commands().enumerate().map(
      |(index, command)| {
        let input_style = match selected {
          Some(selected) if selected == index => Style::CyanBold,
          _ => Style::Gray,
        };

        LineComponent::from([
          Span::styled(command.input(), input_style),
          Span::styled("  ", Style::DarkGray),
          Span::styled(command.description(), Style::DarkGray),
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
      lines[3],
      LineComponent::from([
        Span::styled("/clear", Style::CyanBold),
        Span::styled("  ", Style::DarkGray),
        Span::styled("Clear the transcript", Style::DarkGray),
      ])
    );

    assert_eq!(
      lines[4],
      LineComponent::from([
        Span::styled("/quit", Style::Gray),
        Span::styled("  ", Style::DarkGray),
        Span::styled("Quit kotomori", Style::DarkGray),
      ])
    );
  }
}
