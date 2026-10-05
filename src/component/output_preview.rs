use super::*;

#[derive(Debug)]
pub(crate) struct OutputPreviewComponent {
  pub(crate) limit: usize,
  pub(crate) output: String,
}

impl Component for OutputPreviewComponent {
  fn render(&self, width: u16) -> Vec<LineComponent> {
    let output = self
      .output
      .lines()
      .filter(|line| !line.is_empty())
      .collect::<Vec<_>>();

    let mut lines = output
      .iter()
      .take(self.limit)
      .map(|line| LineComponent::raw(*line).ellipsize(width))
      .collect::<Vec<_>>();

    let omitted = output.len().saturating_sub(self.limit);

    if omitted > 0 {
      lines.extend(
        LineComponent::from([Span::styled(
          format!(
            "... {omitted} more {}",
            if omitted == 1 { "line" } else { "lines" }
          ),
          Style::Muted,
        )])
        .render(width),
      );
    }

    lines
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn renders_limited_non_empty_lines() {
    #[track_caller]
    fn case(limit: usize, expected: &[LineComponent]) {
      assert_eq!(
        OutputPreviewComponent {
          limit,
          output: "\n👩‍💻foobar\n\nbar\nbaz\n".into()
        }
        .render(5),
        expected,
      );
    }

    case(
      0,
      &[
        LineComponent::from([Span::styled("... 3", Style::Muted)]),
        LineComponent::from([Span::styled(" more", Style::Muted)]),
        LineComponent::from([Span::styled(" line", Style::Muted)]),
        LineComponent::from([Span::styled("s", Style::Muted)]),
      ],
    );

    case(
      2,
      &[
        LineComponent::raw("👩‍💻..."),
        LineComponent::raw("bar"),
        LineComponent::from([Span::styled("... 1", Style::Muted)]),
        LineComponent::from([Span::styled(" more", Style::Muted)]),
        LineComponent::from([Span::styled(" line", Style::Muted)]),
      ],
    );

    case(
      3,
      &[
        LineComponent::raw("👩‍💻..."),
        LineComponent::raw("bar"),
        LineComponent::raw("baz"),
      ],
    );
  }
}
