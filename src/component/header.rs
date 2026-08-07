use super::*;

#[derive(Debug)]
pub(crate) struct HeaderComponent {
  first_draw_duration: Option<Duration>,
}

impl HeaderComponent {
  pub(crate) fn new(first_draw_duration: Option<Duration>) -> Self {
    Self {
      first_draw_duration,
    }
  }
}

impl Component for HeaderComponent {
  fn render(&self, _width: u16) -> Vec<LineComponent> {
    let mut spans = vec![
      Span::styled(env!("CARGO_PKG_NAME"), Style::CyanBold),
      Span::raw("  "),
      Span::styled(env!("CARGO_PKG_VERSION"), Style::DarkGray),
    ];

    if let Some(duration) = self.first_draw_duration {
      spans.extend([
        Span::styled(" · ", Style::DarkGray),
        Span::styled(
          format!("first draw {}ms", duration.as_millis()),
          Style::DarkGray,
        ),
      ]);
    }

    vec![LineComponent::from(spans)]
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn rendering_first_draw_duration() {
    assert_eq!(
      HeaderComponent::new(Some(Duration::from_millis(42))).render(80),
      [LineComponent::from([
        Span::styled(env!("CARGO_PKG_NAME"), Style::CyanBold),
        Span::raw("  "),
        Span::styled(env!("CARGO_PKG_VERSION"), Style::DarkGray),
        Span::styled(" · ", Style::DarkGray),
        Span::styled("first draw 42ms", Style::DarkGray),
      ])],
    );
  }
}
