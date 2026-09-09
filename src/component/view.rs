use super::*;

#[derive(Debug)]
pub(crate) struct ViewComponent<'a> {
  first_draw_duration: Option<Duration>,
  screen: &'a Screen,
}

impl<'a> ViewComponent<'a> {
  fn layout(lines: Vec<LineComponent>, width: u16) -> Vec<LineComponent> {
    let content_width = width.saturating_sub(4).max(1);

    lines
      .into_iter()
      .flat_map(|line| {
        if line.is_blank() {
          vec![line]
        } else {
          line.render_prefixed(content_width, &Span::raw("  "))
        }
      })
      .flat_map(|line| line.render(width))
      .collect()
  }

  pub(crate) fn new(
    screen: &'a Screen,
    first_draw_duration: Option<Duration>,
  ) -> Self {
    Self {
      first_draw_duration,
      screen,
    }
  }
}

impl Component for ViewComponent<'_> {
  fn render(&self, width: u16) -> Vec<LineComponent> {
    let content_width = width.saturating_sub(4).max(1);

    let lines = match self.screen {
      Screen::Quit => Vec::new(),
      Screen::Resume(picker) => {
        ResumePickerComponent::new(picker).render(content_width)
      }
      Screen::Session(state) => once(LineComponent::blank())
        .chain(
          HeaderComponent::new(self.first_draw_duration).render(content_width),
        )
        .chain(once(LineComponent::blank()))
        .chain(HintComponent.render(content_width))
        .chain(once(LineComponent::blank()))
        .chain(
          TranscriptComponent::new(
            &state.session.transcript,
            state.active_run(),
          )
          .render(content_width),
        )
        .chain(
          QueuedInputsComponent {
            inputs: state.queued_inputs(),
          }
          .render(content_width),
        )
        .chain(match state.approval() {
          Some(request) => {
            ApprovalPromptComponent::new(request).render(content_width)
          }
          None => ComposerComponent {
            composer: &state.composer,
          }
          .render(content_width),
        })
        .chain(once(LineComponent::blank()))
        .chain(
          FooterComponent::new(&state.model, &state.directory)
            .render(content_width),
        )
        .collect(),
    };

    Self::layout(lines, width)
  }
}

#[cfg(test)]
mod tests {
  use {super::*, unicode_width::UnicodeWidthStr};

  #[test]
  fn composer_renders_while_agent_is_active() {
    let mut state = State::new(&Settings {
      model: "mock:local".parse().unwrap(),
      prompt: Some("foo".into()),
      yolo: false,
    })
    .unwrap();

    state.handle_event(Event::Action(Action::Submit));

    assert!(state.active_run().is_some());

    let screen = Screen::Session(Box::new(state));

    assert!(
      ViewComponent::new(&screen, None)
        .render(80)
        .iter()
        .any(|line| line.to_string().contains("mock · local ·"))
    );
  }

  #[test]
  fn content_has_two_columns_of_side_padding() {
    let state = State::new(&Settings {
      model: "mock:local".parse().unwrap(),
      prompt: None,
      yolo: false,
    })
    .unwrap();

    let lines =
      ViewComponent::new(&Screen::Session(Box::new(state)), None).render(20);

    for line in lines.into_iter().filter(|line| !line.is_blank()) {
      let spans = Vec::<Span>::from(line);

      let width = spans
        .iter()
        .map(|span| UnicodeWidthStr::width(span.text.as_str()))
        .sum::<usize>();

      assert!(width <= 18);

      assert_eq!(
        spans
          .first()
          .unwrap()
          .text
          .chars()
          .take(2)
          .collect::<String>(),
        "  ",
      );
    }
  }

  #[test]
  fn footer_renders_below_approval_prompt() {
    let mut state = State::new(&Settings {
      model: "mock:local".parse().unwrap(),
      prompt: Some("foo".into()),
      yolo: false,
    })
    .unwrap();

    let (request, _response_receiver) = ApprovalRequest::new(ToolInvocation {
      id: "foo".into(),
      kind: ToolInvocationKind::Command(CommandTool {
        command: "bar".into(),
        cwd: None,
      }),
    });

    state.handle_event(Event::Action(Action::Submit));
    state.handle_event(Event::Agent {
      event: AgentEvent::ToolApprovalRequest(request),
      run_id: 0,
    });

    let lines =
      ViewComponent::new(&Screen::Session(Box::new(state)), None).render(80);

    let approval = lines
      .iter()
      .position(|line| line.to_string().contains("deny"))
      .unwrap();

    let footer = lines
      .iter()
      .position(|line| line.to_string().contains("mock · local ·"))
      .unwrap();

    assert!(lines[footer - 1].is_blank());

    assert_eq!(footer, approval + 2);
  }

  #[test]
  fn footer_renders_below_command_menu() {
    let state = State::new(&Settings {
      model: "mock:local".parse().unwrap(),
      prompt: Some("/".into()),
      yolo: false,
    })
    .unwrap();

    let lines =
      ViewComponent::new(&Screen::Session(Box::new(state)), None).render(80);

    let command = lines
      .iter()
      .position(|line| line.to_string().contains("/quit"))
      .unwrap();

    let footer = lines
      .iter()
      .position(|line| line.to_string().contains("mock · local ·"))
      .unwrap();

    assert!(footer > command + 1);
    assert!(lines[footer - 1].is_blank());
  }

  #[test]
  fn layout_preserves_styles_when_wide_characters_exceed_content_width() {
    assert_eq!(
      ViewComponent::layout(
        vec![LineComponent::from([Span::styled("界", Style::Accent)])],
        3,
      ),
      [
        LineComponent::raw("  "),
        LineComponent::from([Span::styled("界", Style::Accent)]),
      ],
    );
  }

  #[test]
  fn layout_wraps_padding_at_narrow_widths() {
    #[track_caller]
    fn case(width: u16, expected: &[&str]) {
      assert_eq!(
        ViewComponent::layout(
          vec![LineComponent::blank(), LineComponent::raw("foo")],
          width,
        ),
        expected
          .iter()
          .copied()
          .map(LineComponent::raw)
          .collect::<Vec<_>>(),
      );
    }

    case(0, &["", " ", " ", "f", " ", " ", "o", " ", " ", "o"]);
    case(1, &["", " ", " ", "f", " ", " ", "o", " ", " ", "o"]);
    case(2, &["", "  ", "f", "  ", "o", "  ", "o"]);
    case(3, &["", "  f", "  o", "  o"]);
    case(4, &["", "  f", "  o", "  o"]);
    case(5, &["", "  f", "  o", "  o"]);
    case(6, &["", "  fo", "  o"]);
    case(7, &["", "  foo"]);
  }
}
