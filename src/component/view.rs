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
    let padding = PaddingComponent { padding: 2 };

    padding.layout(dimensions.width, |width| match self.screen {
      Screen::Quit => Vec::new(),
      Screen::Resume(picker) => {
        ResumePickerComponent { picker }.layout(Dimensions {
          width,
          ..dimensions
        })
      }
      Screen::Session(state) => SessionComponent {
        first_draw_duration: self.first_draw_duration,
        state,
      }
      .render(width),
    })
  }
}
