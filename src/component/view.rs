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

  pub(crate) fn render(&self, dimensions: Dimensions) -> Vec<LineComponent> {
    let stack = match self.screen {
      Screen::Quit => StackComponent::default(),
      Screen::Resume(picker) => StackComponent::default()
        .push(ResumePickerComponent::new(picker, dimensions.height)),
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

    PaddingComponent::new(stack, 2).render(dimensions.width)
  }
}
