use super::*;

/// Chooses the cheapest terminal operation that can present the next frame.
///
/// The planner is conservative. It only returns a patch when the previous
/// frame, terminal dimensions, viewport, and diff all describe a region that
/// can be safely reached and rewritten. Otherwise it asks the renderer to clear
/// or redraw so that stale terminal rows are not left behind.
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
        previous_viewport: presented.previous_viewport(next),
      };
    };

    let previous_viewport = presented.previous_viewport(next);

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
}
