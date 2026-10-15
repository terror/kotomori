use super::*;

#[derive(Debug)]
pub(crate) struct ListComponent<C> {
  pub(crate) items: Vec<C>,
  pub(crate) selected: Option<usize>,
}

impl<C: Component> ListComponent<C> {
  pub(crate) fn layout(
    &self,
    dimensions: Dimensions,
    scroll: &mut Scroll,
  ) -> Vec<LineComponent> {
    let (rows, focus) = self.items.iter().enumerate().fold(
      (Vec::new(), 0..0),
      |(mut rows, focus), (index, item)| {
        let start = rows.len();

        rows.extend(item.render(dimensions.width));

        let focus = if self.selected == Some(index) {
          start..rows.len()
        } else {
          focus
        };

        (rows, focus)
      },
    );

    scroll.update(focus, dimensions.height, rows.len());

    ScrollComponent {
      height: dimensions.height,
      offset: scroll.offset,
      rows,
    }
    .layout()
  }
}
