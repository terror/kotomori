use super::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum RenderPlan {
  Full,
  Patch(ChangedRange),
}

impl RenderPlan {
  pub(crate) fn between(
    presented: &PresentedFrame,
    next: &Frame,
  ) -> Option<Self> {
    if presented.frame.dimensions != next.dimensions {
      return Some(Self::Full);
    }

    let changed = ChangedRange::between(&presented.frame.lines, &next.lines)?;

    if changed.first < presented.viewport_top {
      return Some(Self::Full);
    }

    if changed.first >= next.len() && next.last_row() < presented.viewport_top {
      return Some(Self::Full);
    }

    if presented.frame.len().saturating_sub(next.len()) > next.dimensions.height
    {
      return Some(Self::Full);
    }

    Some(Self::Patch(changed))
  }
}
