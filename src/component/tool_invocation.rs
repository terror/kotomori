use super::*;

#[derive(Debug)]
pub(crate) struct ToolInvocationComponent<'a> {
  pub(crate) invocation: ToolInvocation,
  pub(crate) result: Option<&'a ToolResult>,
}

impl ToolInvocationComponent<'_> {
  const OUTPUT_LIMIT: usize = 3;

  fn details(&self) -> Vec<(&'static str, String)> {
    let mut details = self.invocation.kind.details();

    let exit_status = self.result.and_then(|result| result.exit_status);

    if matches!(exit_status, Some(status) if status != 0) {
      details.push(("exit", exit_status.unwrap().to_string()));
    }

    details
  }
}

impl Component for ToolInvocationComponent<'_> {
  fn render(&self, width: u16) -> Vec<LineComponent> {
    let (symbol, symbol_style, tense) = match self.result {
      Some(result) if result.outcome == ToolOutcome::Failure => {
        ("●", Style::Danger, ToolActionTense::Failed)
      }
      Some(_) => ("●", Style::Success, ToolActionTense::Completed),
      None => ("●", Style::Accent, ToolActionTense::Progressive),
    };

    StackComponent::default()
      .push(LineComponent::from([
        Span::styled(symbol, symbol_style),
        Span::raw(" "),
        Span::raw(self.invocation.title(tense)),
      ]))
      .extend(self.details().into_iter().map(|(label, value)| {
        LineComponent::from([
          Span::styled(format!("{label} "), Style::Muted),
          Span::raw(value),
        ])
        .gutter(Span::styled("  │ ", Style::Muted))
      }))
      .push(self.result.and_then(ToolResult::output).map(|output| {
        OutputPreviewComponent {
          limit: Self::OUTPUT_LIMIT,
          output,
        }
        .gutter(Span::styled("  │ ", Style::Muted))
      }))
      .render(width)
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
      ToolInvocationComponent {
        invocation,
        result: Some(&result)
      }
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
