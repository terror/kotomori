use super::*;

#[derive(Debug)]
pub(crate) struct RenderPlanner<'a> {
  max_lines_rendered: usize,
  presented: Option<&'a PresentedFrame>,
}

impl<'a> RenderPlanner<'a> {
  pub(crate) fn new(
    max_lines_rendered: usize,
    presented: Option<&'a PresentedFrame>,
  ) -> Self {
    Self {
      max_lines_rendered,
      presented,
    }
  }

  pub(crate) fn plan(self, next: &Frame) -> RenderPlan<'a> {
    let Some(presented) = self.presented else {
      return RenderPlan::Full { clear: false };
    };

    if presented.frame.dimensions.width != next.dimensions.width {
      return RenderPlan::Full { clear: true };
    }

    if presented.frame.dimensions.height != next.dimensions.height {
      return RenderPlan::Full { clear: true };
    }

    if next.len() < self.max_lines_rendered
      && env::var_os("KOTOMORI_CLEAR_ON_SHRINK").is_some()
    {
      return RenderPlan::Full { clear: true };
    }

    let Some(diff) = Diff::between(&presented.frame, next) else {
      return RenderPlan::NoOperation {
        previous: presented,
        previous_viewport: Self::previous_viewport(presented, next),
      };
    };

    let previous_viewport = Self::previous_viewport(presented, next);

    if diff.changed.first < previous_viewport.top() {
      return RenderPlan::Full { clear: true };
    }

    if diff.is_pure_tail_delete() && next.last_row() < previous_viewport.top() {
      return RenderPlan::Full { clear: true };
    }

    if diff.deleted_tail_len() > next.dimensions.height() {
      return RenderPlan::Full { clear: true };
    }

    let patch = if diff.is_pure_tail_delete() {
      PatchPlan::ClearDeletedTail { diff }
    } else {
      let Some(writable_range) = diff.writable_range() else {
        return RenderPlan::Full { clear: true };
      };

      PatchPlan::Update {
        diff,
        writable_range,
      }
    };

    RenderPlan::Patch {
      previous: presented,
      previous_viewport,
      patch,
    }
  }

  fn previous_viewport(presented: &PresentedFrame, next: &Frame) -> Viewport {
    let height = next.dimensions.height();

    if presented.frame.dimensions.height == next.dimensions.height {
      Viewport::new(presented.viewport.top(), height)
    } else {
      Viewport::anchored_to_bottom(
        presented
          .viewport
          .top()
          .saturating_add(presented.frame.dimensions.height()),
        height,
      )
    }
  }
}
