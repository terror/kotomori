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
    match self.screen {
      Screen::Quit => Vec::new(),
      Screen::Resume(picker) => {
        ResumePickerComponent::new(picker).render(width)
      }
      Screen::Session(state) => once(LineComponent::blank())
        .chain(HeaderComponent::new(self.first_draw_duration).render(width))
        .chain(once(LineComponent::blank()))
        .chain(HintComponent.render(width))
        .chain(once(LineComponent::blank()))
        .chain(TranscriptComponent::new(&state.transcript).render(width))
        .chain(match &state.input_mode {
          InputMode::Approval(request) => {
            ApprovalPromptComponent::new(request).render(width)
          }
          InputMode::Compose => {
            ComposerComponent::new(&state.composer).render(width)
          }
        })
        .chain(
          FooterComponent::new(&state.model, &state.directory).render(width),
        )
        .collect(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn composer_renders_while_agent_is_active() {
    let mut state = State::new(&Settings {
      model: "mock:local".parse().unwrap(),
      prompt: Some("foo".into()),
      yolo: false,
    })
    .unwrap();

    state.handle_event(Event::Action(Action::Submit));

    assert!(state.transcript.is_agent_active());

    let screen = Screen::Session(Box::new(state));

    assert!(
      ViewComponent::new(&screen, None)
        .render(80)
        .iter()
        .any(|line| line.to_string().contains("mock · local ·"))
    );
  }

  #[test]
  fn footer_renders_below_approval_prompt() {
    let mut state = State::new(&Settings {
      model: "mock:local".parse().unwrap(),
      prompt: None,
      yolo: false,
    })
    .unwrap();

    let (request, _response_receiver) = ApprovalRequest::new(ToolInvocation {
      id: "foo".into(),
      kind: ToolInvocationKind::Command(CommandTool {
        arguments: Vec::new(),
        cwd: None,
        program: "bar".into(),
      }),
    });

    state.input_mode = InputMode::Approval(request);

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

    assert!(footer > approval);
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

    assert!(footer > command);
  }
}
