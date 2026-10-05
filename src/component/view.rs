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
      Screen::Session(state) => StackComponent::default()
        .push(SessionComponent::layout(state, self.first_draw_duration)),
    };

    stack.padded(padding).render(dimensions.width)
  }
}
