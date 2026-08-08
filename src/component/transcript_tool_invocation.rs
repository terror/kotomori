use super::*;

#[derive(Debug)]
pub(crate) struct TranscriptToolInvocationComponent<'a> {
  invocation: &'a ToolInvocation,
  result: Option<&'a ToolResult>,
}

impl<'a> TranscriptToolInvocationComponent<'a> {
  const GUTTER: &'static str = "   │ ";
  const OUTPUT_LIMIT: usize = 3;

  fn details(&self) -> Vec<(&'static str, String)> {
    let mut details = self.invocation.kind.details();

    let exit_status = self.result.and_then(|result| result.exit_status);

    if matches!(exit_status, Some(status) if status != 0) {
      details.push(("exit", exit_status.unwrap().to_string()));
    }

    details
  }

  pub(crate) fn new(
    invocation: &'a ToolInvocation,
    result: Option<&'a ToolResult>,
  ) -> Self {
    Self { invocation, result }
  }

  fn preview(line: &str, width: usize) -> String {
    let (mut preview, mut preview_width) = (String::new(), 0usize);

    for c in line.chars().by_ref() {
      let char_width = UnicodeWidthChar::width(c).unwrap_or(0);

      if preview_width + char_width > width {
        while preview_width.saturating_add(3) > width {
          let Some(c) = preview.pop() else {
            break;
          };

          preview_width = preview_width
            .saturating_sub(UnicodeWidthChar::width(c).unwrap_or(0));
        }

        preview.push_str("...");

        return preview;
      }

      preview.push(c);
      preview_width += char_width;
    }

    preview
  }
}

impl Component for TranscriptToolInvocationComponent<'_> {
  fn render(&self, width: u16) -> Vec<LineComponent> {
    let mut lines = Vec::new();

    let (symbol, symbol_style, title) = match self.result {
      Some(result) if result.is_error() => {
        ("●", Style::RedBold, self.invocation.failed_tense())
      }
      Some(_) => ("●", Style::GreenBold, self.invocation.completed_tense()),
      None => ("●", Style::CyanBold, self.invocation.progressive_tense()),
    };

    lines.push(LineComponent::from([
      Span::raw(" "),
      Span::styled(symbol, symbol_style),
      Span::raw(" "),
      Span::raw(title),
    ]));

    lines.extend(self.details().into_iter().map(|(label, value)| {
      LineComponent::from([
        Span::styled(Self::GUTTER, Style::DarkGray),
        Span::styled(format!("{label} "), Style::DarkGray),
        Span::raw(value),
      ])
    }));

    if let Some(output) = self.result.and_then(ToolResult::output) {
      let output_width = usize::from(width.saturating_sub(5).max(8));

      let output_lines = output
        .lines()
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();

      lines.extend(output_lines.iter().take(Self::OUTPUT_LIMIT).map(|line| {
        LineComponent::from([
          Span::styled(Self::GUTTER, Style::DarkGray),
          Span::styled(Self::preview(line, output_width), Style::DarkGray),
        ])
      }));

      let omitted = output_lines.len().saturating_sub(Self::OUTPUT_LIMIT);

      if omitted > 0 {
        lines.push(LineComponent::from([
          Span::styled(Self::GUTTER, Style::DarkGray),
          Span::styled(
            format!(
              "... {omitted} more {}",
              if omitted == 1 { "line" } else { "lines" }
            ),
            Style::DarkGray,
          ),
        ]));
      }
    }

    lines
  }
}
