use super::*;

#[derive(Debug)]
pub(crate) struct TranscriptToolInvocationComponent<'a> {
  invocation: ToolInvocation,
  result: Option<&'a ToolResult>,
}

impl<'a> TranscriptToolInvocationComponent<'a> {
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
    invocation: ToolInvocation,
    result: Option<&'a ToolResult>,
  ) -> Self {
    Self { invocation, result }
  }
}

impl Component for TranscriptToolInvocationComponent<'_> {
  fn render(&self, width: u16) -> Vec<LineComponent> {
    let (symbol, symbol_style, tense) = match self.result {
      Some(result) if result.is_error() => {
        ("●", Style::Danger, ToolActionTense::Failed)
      }
      Some(_) => ("●", Style::Success, ToolActionTense::Completed),
      None => ("●", Style::Accent, ToolActionTense::Progressive),
    };

    let stack = StackComponent::default()
      .push(LineComponent::from([
        Span::styled(symbol, symbol_style),
        Span::raw(" "),
        Span::raw(self.invocation.title(tense)),
      ]))
      .push(GutterComponent::new(
        LinesComponent::new(self.details().into_iter().map(
          |(label, value)| {
            LineComponent::from([
              Span::styled(format!("{label} "), Style::Muted),
              Span::raw(value),
            ])
          },
        )),
        Span::styled("  │ ", Style::Muted),
      ));

    let stack = if let Some(output) = self.result.and_then(ToolResult::output) {
      stack.push(GutterComponent::new(
        OutputPreviewComponent::new(output, Self::OUTPUT_LIMIT),
        Span::styled("  │ ", Style::Muted),
      ))
    } else {
      stack
    };

    stack.render(width)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn wraps_details_inside_gutter() {
    let invocation = ToolInvocation::new(
      "foo",
      ToolInvocationKind::Command(CommandTool {
        command: "bar".into(),
        cwd: Some("foobar".into()),
      }),
    );

    let result = ToolResult {
      outcome: ToolOutcome::Success,
      ..Default::default()
    };

    assert_eq!(
      TranscriptToolInvocationComponent::new(invocation, Some(&result))
        .render(10),
      [
        LineComponent::from([
          Span::styled("●", Style::Success),
          Span::raw(" "),
          Span::raw("Ran bar"),
        ]),
        LineComponent::from([
          Span::styled("  │ ", Style::Muted),
          Span::styled("cwd ", Style::Muted),
          Span::raw("fo"),
        ]),
        LineComponent::from([
          Span::styled("  │ ", Style::Muted),
          Span::raw("obar"),
        ]),
      ],
    );
  }
}
