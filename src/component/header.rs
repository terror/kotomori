use super::*;

#[derive(Debug)]
pub(crate) struct HeaderComponent;

impl Component for HeaderComponent {
  fn render(&self, _width: u16) -> Vec<Line> {
    vec![Line::from([
      Span::styled(env!("CARGO_PKG_NAME"), Style::CyanBold),
      Span::raw("  "),
      Span::styled(env!("CARGO_PKG_VERSION"), Style::DarkGray),
    ])]
  }
}
