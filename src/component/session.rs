use super::*;

#[derive(Debug)]
pub(crate) struct SessionComponent<'a> {
  stack: StackComponent<'a>,
}

impl<'a> SessionComponent<'a> {
  pub(crate) fn layout(
    state: &'a State,
    first_draw_duration: Option<Duration>,
  ) -> Self {
    let stack = StackComponent::default()
      .gap(1)
      .push(HeaderComponent {
        first_draw_duration,
      })
      .push(HintComponent)
      .push(TranscriptComponent {
        reasoning_expanded: state.reasoning_expanded,
        run: state.run.as_ref(),
        state: &state.session.transcript,
      })
      .push(QueuedInputsComponent {
        inputs: &state.queued_inputs,
      });

    let stack = match state.run.as_ref().and_then(|run| run.approval.as_ref()) {
      Some(request) => stack.push(ApprovalPromptComponent { request }),
      None => stack.push(ComposerComponent {
        composer: &state.composer,
      }),
    };

    Self {
      stack: StackComponent::default().push(LineComponent::blank()).push(
        stack.push(FooterComponent {
          directory: &state.session.settings.directory,
          model: &state.session.settings.model,
        }),
      ),
    }
  }
}

impl Component for SessionComponent<'_> {
  fn render(&self, width: u16) -> Vec<LineComponent> {
    self.stack.render(width)
  }
}
