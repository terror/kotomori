use super::*;

#[derive(Debug)]
pub(crate) struct ViewComponent<'a> {
  pub(crate) first_draw_duration: Option<Duration>,
  pub(crate) screen: &'a mut Screen,
}

impl ViewComponent<'_> {
  pub(crate) fn layout(
    &mut self,
    dimensions: Dimensions,
  ) -> Vec<LineComponent> {
    let padding = 2.min(dimensions.width.saturating_sub(1) / 2);

    let stack = match self.screen {
      Screen::Quit => StackComponent::default(),
      Screen::Resume(picker) => {
        StackComponent::default().push(ResumePickerComponent::layout(
          picker,
          Dimensions {
            width: dimensions.width - padding * 2,
            ..dimensions
          },
        ))
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
      padding,
    }
    .render(dimensions.width)
  }
}
