use super::*;

#[derive(Debug)]
pub(crate) struct ViewComponent<'a> {
  pub(crate) first_draw_duration: Option<Duration>,
  pub(crate) screen: &'a Screen,
}

impl ViewComponent<'_> {
  pub(crate) fn render(&self, dimensions: Dimensions) -> Vec<LineComponent> {
    let stack = match self.screen {
      Screen::Quit => StackComponent::default(),
      Screen::Resume(picker) => {
        StackComponent::default().push(ResumePickerComponent {
          height: dimensions.height,
          picker,
        })
      }
      Screen::Session(state) => {
        let stack = StackComponent::default()
          .push(LineComponent::blank())
          .push_spaced(HeaderComponent {
            first_draw_duration: self.first_draw_duration,
          })
          .push_spaced(HintComponent)
          .push(TranscriptComponent {
            reasoning_expanded: state.reasoning_expanded,
            run: state.run.as_ref(),
            state: &state.session.transcript,
          })
          .push(QueuedInputsComponent {
            inputs: &state.queued_inputs,
          });

        let stack =
          match state.run.as_ref().and_then(|run| run.approval.as_ref()) {
            Some(request) => stack.push(ApprovalPromptComponent { request }),
            None => stack.push(ComposerComponent {
              composer: &state.composer,
            }),
          };

        stack.push(LineComponent::blank()).push(FooterComponent {
          directory: &state.session.settings.directory,
          model: &state.session.settings.model,
        })
      }
    };

    PaddingComponent {
      component: stack,
      padding: 2,
    }
    .render(dimensions.width)
  }
}
