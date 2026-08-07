use super::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum RenderPlan {
  Full { clear: bool },
  NoOperation,
  Patch { diff: Diff },
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

    let Some(diff) = Diff::between(&presented.frame, next) else {
      return Self::NoOperation;
    };

    if diff.changed.first < presented.viewport.top {
      return Self::Full { clear: true };
    }

    if diff.is_pure_tail_delete() && next.last_row() < presented.viewport.top {
      return Self::Full { clear: true };
    }

    if diff.deleted_tail_len() > next.dimensions.height {
      return Self::Full { clear: true };
    }

    Self::Patch { diff }
  }
}
