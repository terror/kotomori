use super::*;

#[derive(Debug)]
pub(crate) struct ViewComponent<'a> {
  first_draw_duration: Option<Duration>,
  screen: &'a Screen,
}

impl<'a> ViewComponent<'a> {
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
    let stack = match self.screen {
      Screen::Quit => StackComponent::default(),
      Screen::Resume(picker) => {
        StackComponent::default().push(ResumePickerComponent::new(picker))
      }
      Screen::Session(state) => {
        let stack = StackComponent::default()
          .push(LineComponent::blank())
          .push_spaced(HeaderComponent::new(self.first_draw_duration))
          .push_spaced(HintComponent)
          .push(TranscriptComponent::new(
            state.transcript(),
            state.active_run(),
          ))
          .push(QueuedInputsComponent {
            inputs: state.queued_inputs(),
          });

        let stack = match state.approval() {
          Some(request) => stack.push(ApprovalPromptComponent::new(request)),
          None => stack.push(ComposerComponent {
            composer: state.composer(),
          }),
        };

        stack
          .push(LineComponent::blank())
          .push(FooterComponent::new(state.model(), state.directory()))
      }
    };

    PaddingComponent::new(stack, 2).render(width)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn composer_renders_while_agent_is_active() {
    let mut state = State::new(
      Database::new().unwrap(),
      Session::new(&Settings {
        model: "mock:local".parse().unwrap(),
        prompt: Some("foo".into()),
        yolo: false,
      })
      .unwrap(),
    )
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
    let state = State::new(
      Database::new().unwrap(),
      Session::new(&Settings {
        model: "mock:local".parse().unwrap(),
        prompt: Some("/".into()),
        yolo: false,
      })
      .unwrap(),
    )
    .unwrap();

    for screen in [
      Screen::Session(Box::new(state)),
      Screen::Resume(ResumePicker::new(Vec::new())),
      Screen::Resume(ResumePicker::new(vec![SessionSummary {
        directory: "foo".into(),
        id: 0,
        model: "mock:bar".parse().unwrap(),
        title: Some("baz".into()),
        updated_at: u64::MAX,
      }])),
    ] {
      for width in 0..=20 {
        let lines = ViewComponent::new(&screen, None).render(width);

        for line in lines.into_iter().filter(|line| !line.is_blank()) {
          let spans = Vec::<Span>::from(line);

          let line_width = spans
            .iter()
            .map(|span| UnicodeWidthStr::width(span.text.as_str()))
            .sum::<usize>();

          assert!(line_width <= usize::from(width));

          if width >= 5 {
            assert!(line_width <= usize::from(width - 2));
            assert_eq!(spans.first().unwrap().text, "  ");
          }
        }
      }
    }
  }

  #[test]
  fn footer_renders_below_approval_prompt() {
    let mut state = State::new(
      Database::new().unwrap(),
      Session::new(&Settings {
        model: "mock:local".parse().unwrap(),
        prompt: Some("foo".into()),
        yolo: false,
      })
      .unwrap(),
    )
    .unwrap();

    let (request, _response_receiver) =
      ApprovalRequest::new(ToolInvocation::new(
        "foo",
        ToolInvocationKind::Command(CommandTool {
          command: "bar".into(),
          cwd: None,
        }),
      ));

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
    let state = State::new(
      Database::new().unwrap(),
      Session::new(&Settings {
        model: "mock:local".parse().unwrap(),
        prompt: Some("/".into()),
        yolo: false,
      })
      .unwrap(),
    )
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
}
