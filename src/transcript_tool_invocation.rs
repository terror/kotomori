use super::*;

#[derive(Debug)]
pub(crate) struct TranscriptToolInvocation<'a> {
  invocation: &'a ToolInvocation,
  result: Option<&'a ToolResult>,
}

impl<'a> TranscriptToolInvocation<'a> {
  const GUTTER: &'static str = "   │ ";
  const OUTPUT_LIMIT: usize = 3;

  fn details(&self) -> Vec<(&'static str, String)> {
    let mut details = self.invocation.kind.details();

    if let Some(exit_status) = self
      .result
      .and_then(ToolResult::exit_status)
      .filter(|exit_status| *exit_status != 0)
    {
      details.push(("exit", exit_status.to_string()));
    }

    details
  }

  pub(crate) fn new(
    invocation: &'a ToolInvocation,
    result: Option<&'a ToolResult>,
  ) -> Self {
    Self { invocation, result }
  }

  fn output_line(value: impl Into<String>) -> Line {
    vec![
      Span::styled(Self::GUTTER, Style::DarkGray),
      Span::styled(value.into(), Style::DarkGray),
    ]
    .into()
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

  fn status(&self) -> (&'static str, Style, String) {
    match self.result {
      Some(result) if result.is_error() => {
        ("●", Style::RedBold, self.invocation.failed_tense())
      }
      Some(_) => ("●", Style::GreenBold, self.invocation.completed_tense()),
      None => ("●", Style::CyanBold, self.invocation.progressive_tense()),
    }
  }
}

impl Component for TranscriptToolInvocation<'_> {
  fn render(&self, width: u16) -> Vec<Line> {
    let mut lines = Vec::new();

    let (symbol, symbol_style, title) = self.status();

    lines.push(Line::blank());

    lines.push(
      vec![
        Span::raw(" "),
        Span::styled(symbol, symbol_style),
        Span::raw(" "),
        Span::raw(title),
      ]
      .into(),
    );

    lines.extend(self.details().into_iter().map(|(label, value)| {
      vec![
        Span::styled(Self::GUTTER, Style::DarkGray),
        Span::styled(format!("{label} "), Style::DarkGray),
        Span::raw(value),
      ]
      .into()
    }));

    if let Some(output) = self.result.and_then(ToolResult::output) {
      let output_width = usize::from(width.saturating_sub(5).max(8));

      let output_lines = output
        .lines()
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();

      lines.extend(
        output_lines
          .iter()
          .take(Self::OUTPUT_LIMIT)
          .map(|line| Self::output_line(Self::preview(line, output_width))),
      );

      let omitted = output_lines.len().saturating_sub(Self::OUTPUT_LIMIT);

      if omitted > 0 {
        lines.push(Self::output_line(format!(
          "... {omitted} more {}",
          if omitted == 1 { "line" } else { "lines" }
        )));
      }
    }

    lines.push(Line::blank());

    lines
  }
}
