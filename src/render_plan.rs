use super::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum RenderPlan {
  Full,
  Patch(Patch),
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

    let prepend_line_feed =
      changed.first > 0 && changed.first == presented.frame.len();

    let move_target_row = if prepend_line_feed {
      changed.first.saturating_sub(1)
    } else {
      changed.first
    };

    let viewport_bottom = presented
      .viewport_top
      .saturating_add(next.dimensions.height.saturating_sub(1));

    debug_assert!(move_target_row <= viewport_bottom);

    let viewport_top = presented.viewport_top.max(
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

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn append_past_viewport_advances_viewport() {
    let presented = Frame::new(
      vec!["foo".into(), "bar".into()],
      Dimensions {
        height: 1,
        width: 80,
      },
    )
    .into();

    let next = Frame::new(
      vec!["foo".into(), "bar".into(), "baz".into()],
      Dimensions {
        height: 1,
        width: 80,
      },
    );

    assert_eq!(
      RenderPlan::between(&presented, &next),
      Some(RenderPlan::Patch(Patch {
        changed: ChangedRange { first: 2, last: 2 },
        move_target_row: 1,
        prepend_line_feed: true,
        viewport_top: 2,
      })),
    );
  }

  #[test]
  fn append_starts_from_previous_row_and_feeds_line() {
    let presented = Frame::new(
      vec!["foo".into()],
      Dimensions {
        height: 24,
        width: 80,
      },
    )
    .into();

    let next = Frame::new(
      vec!["foo".into(), "bar".into()],
      Dimensions {
        height: 24,
        width: 80,
      },
    );

    assert_eq!(
      RenderPlan::between(&presented, &next),
      Some(RenderPlan::Patch(Patch {
        changed: ChangedRange { first: 1, last: 1 },
        move_target_row: 0,
        prepend_line_feed: true,
        viewport_top: 0,
      })),
    );
  }

  #[test]
  fn replacement_starts_from_first_changed_row() {
    let presented = Frame::new(
      vec!["foo".into(), "bar".into(), "baz".into()],
      Dimensions {
        height: 24,
        width: 80,
      },
    )
    .into();

    let next = Frame::new(
      vec!["foo".into(), "qux".into(), "baz".into()],
      Dimensions {
        height: 24,
        width: 80,
      },
    );

    assert_eq!(
      RenderPlan::between(&presented, &next),
      Some(RenderPlan::Patch(Patch {
        changed: ChangedRange { first: 1, last: 1 },
        move_target_row: 1,
        prepend_line_feed: false,
        viewport_top: 0,
      })),
    );
  }
}
