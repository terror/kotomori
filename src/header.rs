use super::*;

#[derive(Debug)]
pub(crate) struct Header;

impl Component for Header {
  fn render(&self, _width: u16) -> Vec<Line> {
    vec![Line::from(vec![
      Span::styled(env!("CARGO_PKG_NAME"), Style::CyanBold),
      Span::raw("  "),
      Span::styled(env!("CARGO_PKG_VERSION"), Style::DarkGray),
    ])]
  }
}
