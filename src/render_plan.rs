use super::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum RenderPlan {
  Full,
  Patch(Patch),
}

impl RenderPlan {
  pub(crate) fn between(current: &Frame, next: &Frame) -> Option<Self> {
    if current.dimensions != next.dimensions {
      return Some(Self::Full);
    }

    let changed = ChangedRange::between(&current.lines, &next.lines)?;

    if changed.first < current.viewport_top {
      return Some(Self::Full);
    }

    if changed.first >= next.len() && next.last_row() < current.viewport_top {
      return Some(Self::Full);
    }

    if current.len().saturating_sub(next.len()) > next.dimensions.height {
      return Some(Self::Full);
    }

    let prepend_line_feed = changed.first > 0 && changed.first == current.len();

    let move_target_row = if prepend_line_feed {
      changed.first.saturating_sub(1)
    } else {
      changed.first
    };

    let viewport_bottom = current
      .viewport_top
      .saturating_add(next.dimensions.height.saturating_sub(1));

    debug_assert!(move_target_row <= viewport_bottom);

    let viewport_top = current.viewport_top.max(
      changed
        .last
        .saturating_add(1)
        .saturating_sub(next.dimensions.height),
    );

    Some(Self::Patch(Patch {
      changed,
      move_target_row,
      prepend_line_feed,
      viewport_top,
    }))
  }
}
