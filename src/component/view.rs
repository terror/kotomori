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
          TranscriptComponent::new(&state.transcript).render(content_width),
        )
        .chain(match &state.input_mode {
          InputMode::Approval(request) => {
            ApprovalPromptComponent::new(request).render(content_width)
          }
          InputMode::Compose => {
            ComposerComponent::new(&state.composer).render(content_width)
          }
        })
        .chain(once(LineComponent::blank()))
        .chain(
          FooterComponent::new(&state.model, &state.directory)
            .render(content_width),
        )
        .collect(),
    };

    lines
      .into_iter()
      .flat_map(|line| line.render(content_width))
      .map(|line| {
        if line.is_blank() {
          line
        } else {
          let mut spans = Vec::<Span>::from(line);
          spans.insert(0, Span::raw("  "));
          LineComponent::from(spans)
        }
      })
      .collect()
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

      assert_eq!(spans.first().unwrap().text, "  ");
    }
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
        command: "bar".into(),
        cwd: None,
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
}
