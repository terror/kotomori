use super::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum RenderPlan {
  Full { clear: bool },
  NoOperation,
  Patch { changed: ChangedRange },
}

impl RenderPlan {
  pub(crate) fn between(
    presented: Option<&PresentedFrame>,
    next: &Frame,
  ) -> Self {
    let Some(presented) = presented else {
      return Self::Full { clear: false };
    };

    if presented.frame.dimensions != next.dimensions {
      return Self::Full { clear: true };
    }

    let Some(changed) =
      ChangedRange::between(&presented.frame.lines, &next.lines)
    else {
      return Self::NoOperation;
    };

    if changed.first < presented.viewport_top {
      return Self::Full { clear: true };
    }

    if changed.first >= next.len() && next.last_row() < presented.viewport_top {
      return Self::Full { clear: true };
    }

    if presented.frame.len().saturating_sub(next.len()) > next.dimensions.height
    {
      return Self::Full { clear: true };
    }

    Self::Patch { changed }
  }
}
