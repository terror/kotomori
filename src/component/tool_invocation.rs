use super::*;

#[derive(Debug)]
pub(crate) struct ToolInvocationComponent<'a> {
  pub(crate) invocation: &'a ToolRequest,
  pub(crate) result: Option<&'a ToolResult>,
  pub(crate) run: Option<&'a Run>,
}

impl ToolInvocationComponent<'_> {
  const OUTPUT_LIMIT: usize = 3;

  fn command(invocation: &ToolRequest) -> Option<CommandArguments> {
    (invocation.function.name.as_str() == "command")
      .then(|| {
        serde_json::from_value(invocation.function.arguments.clone()).ok()
      })
      .flatten()
  }

  pub(crate) fn description(invocation: &ToolRequest) -> String {
    match Self::command(invocation) {
      Some(command) => command.command,
      None => format!(
        "{} {}",
        invocation.function.name, invocation.function.arguments
      ),
    }
  }

  fn details(&self) -> Vec<(&'static str, String)> {
    Self::command(self.invocation)
      .and_then(|command| command.cwd)
      .map(|cwd| ("cwd", cwd.display().to_string()))
      .into_iter()
      .collect()
  }

  fn title(&self, tense: ToolActionTense) -> String {
    let action = match tense {
      ToolActionTense::AwaitingApproval => "Awaiting approval to run",
      ToolActionTense::Completed => "Ran",
      ToolActionTense::Denied => "Denied running",
      ToolActionTense::Failed => "Failed running",
      ToolActionTense::Interrupted => "Interrupted running",
      ToolActionTense::Progressive => "Running",
      ToolActionTense::Queued => "Queued",
      ToolActionTense::Skipped => "Skipped running",
    };

    format!("{action} {}", Self::description(self.invocation))
  }
}

impl Component for ToolInvocationComponent<'_> {
  fn render(&self, width: u16) -> Vec<LineComponent> {
    let (symbol, symbol_style, tense) = match self
      .result
      .map(|result| result.outcome)
    {
      Some(ToolOutcome::Denied) => ("●", Style::Muted, ToolActionTense::Denied),
      Some(ToolOutcome::Failure) => {
        ("●", Style::Danger, ToolActionTense::Failed)
      }
      Some(ToolOutcome::Interrupted) => {
        ("●", Style::Muted, ToolActionTense::Interrupted)
      }
      Some(ToolOutcome::Skipped) => {
        ("●", Style::Muted, ToolActionTense::Skipped)
      }
      Some(ToolOutcome::Success) => {
        ("●", Style::Success, ToolActionTense::Completed)
      }
      None => match self.run {
        Some(run)
          if run.approval.as_ref().is_some_and(|request| {
            request.invocation.id == self.invocation.id
          }) =>
        {
          ("●", Style::Muted, ToolActionTense::AwaitingApproval)
        }
        Some(run) if run.running_tool.as_ref() == Some(&self.invocation.id) => {
          ("●", Style::Accent, ToolActionTense::Progressive)
        }
        _ => ("●", Style::Muted, ToolActionTense::Queued),
      },
    };

    let gutter = LineComponent::styled("  │ ", Style::Muted);

    let output_width =
      width.saturating_sub(u16::try_from(gutter.width()).unwrap_or(u16::MAX));

    let mut details = self.details();

    let exit_status = self.result.and_then(|result| {
      serde_json::from_value::<CommandOutput>(result.details.clone())
        .ok()
        .and_then(|output| output.exit_status)
    });

    if matches!(exit_status, Some(status) if status != 0) {
      details.push(("exit", exit_status.unwrap().to_string()));
    }

    StackComponent::default()
      .push(LineComponent::from([
        Span::styled(symbol, symbol_style),
        Span::raw(" "),
        Span::raw(self.title(tense)),
      ]))
      .extend(details.into_iter().map(|(label, value)| {
        LineComponent::from([
          Span::styled(format!("{label} "), Style::Muted),
          Span::raw(value),
        ])
        .gutter(gutter.clone())
      }))
      .push(self.result.and_then(ToolResult::output).map(|output| {
        let output = output
          .lines()
          .filter(|line| !line.is_empty())
          .collect::<Vec<_>>();

        let mut lines = output
          .iter()
          .take(Self::OUTPUT_LIMIT)
          .map(|line| LineComponent::raw(*line).ellipsize(output_width))
          .collect::<Vec<_>>();

        let omitted = output.len().saturating_sub(Self::OUTPUT_LIMIT);

        if omitted > 0 {
          lines.push(LineComponent::from([Span::styled(
            format!(
              "... {omitted} more {}",
              if omitted == 1 { "line" } else { "lines" }
            ),
            Style::Muted,
          )]));
        }

        LinesComponent { lines }.gutter(gutter)
      }))
      .render(width)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn renders_limited_non_empty_lines() {
    #[track_caller]
    fn case(output: &str, expected: &[LineComponent]) {
      let invocation = ToolRequest::new(
        CallId::new("foo"),
        ToolFunction {
          arguments: serde_json::json!({"command": "bar"}),
          name: ToolName::new("command").unwrap(),
        },
      );

      let result = ToolResult {
        content: output.into(),
        outcome: ToolOutcome::Success,
        ..Default::default()
      };

      assert_eq!(
        ToolInvocationComponent {
          invocation: &invocation,
          result: Some(&result),
          run: None,
        }
        .render(9),
        once(LineComponent::from([
          Span::styled("●", Style::Success),
          Span::raw(" Ran bar"),
        ]))
        .chain(expected.iter().cloned().map(|line| {
          LineComponent::styled("  │ ", Style::Muted).append(line)
        }))
        .collect::<Vec<_>>(),
      );
    }

    case(
      "\n👩‍💻foobar\n\nbar\nbaz\nfoo\nbar\nbaz\n",
      &[
        LineComponent::raw("👩‍💻..."),
        LineComponent::raw("bar"),
        LineComponent::raw("baz"),
        LineComponent::from([Span::styled("... 3", Style::Muted)]),
        LineComponent::from([Span::styled(" more", Style::Muted)]),
        LineComponent::from([Span::styled(" line", Style::Muted)]),
        LineComponent::from([Span::styled("s", Style::Muted)]),
      ],
    );

    case(
      "\n👩‍💻foobar\n\nbar\nbaz\nfoo\n",
      &[
        LineComponent::raw("👩‍💻..."),
        LineComponent::raw("bar"),
        LineComponent::raw("baz"),
        LineComponent::from([Span::styled("... 1", Style::Muted)]),
        LineComponent::from([Span::styled(" more", Style::Muted)]),
        LineComponent::from([Span::styled(" line", Style::Muted)]),
      ],
    );

    case(
      "\n👩‍💻foobar\n\nbar\nbaz\n",
      &[
        LineComponent::raw("👩‍💻..."),
        LineComponent::raw("bar"),
        LineComponent::raw("baz"),
      ],
    );
  }

  #[test]
  fn wraps_details_inside_gutter() {
    let invocation = ToolRequest::new(
      CallId::new("foo"),
      ToolFunction {
        arguments: serde_json::json!({"command": "bar", "cwd": "foobar"}),
        name: ToolName::new("command").unwrap(),
      },
    );

    let result = ToolResult {
      outcome: ToolOutcome::Success,
      ..Default::default()
    };

    assert_eq!(
      ToolInvocationComponent {
        invocation: &invocation,
        result: Some(&result),
        run: None,
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
