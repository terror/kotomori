use super::*;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[must_use]
pub(crate) struct LineComponent {
  spans: SmallVec<[Span; 6]>,
}

impl LineComponent {
  pub(crate) fn append(mut self, line: impl Into<Self>) -> Self {
    for span in line.into().spans {
      self.push(span);
    }

    self
  }

  pub(crate) fn blank() -> Self {
    Self::default()
  }

  pub(crate) fn clip_end(&self, width: u16) -> Self {
    if width == 0 {
      return Self::blank();
    }

    self.slice(0..Self::fitting_length(Self::graphemes(&self.text()), width))
  }

  pub(crate) fn clip_start(&self, width: u16) -> Self {
    if width == 0 {
      return Self::blank();
    }

    let text = self.text();

    let start =
      text.len() - Self::fitting_length(Self::graphemes(&text).rev(), width);

    self.slice(start..text.len())
  }

  pub(crate) fn ellipsize(&self, width: u16) -> Self {
    if width == 0 {
      return Self::blank();
    }

    if self.width() <= usize::from(width) {
      return self.clone();
    }

    self
      .clip_end(width.saturating_sub(3))
      .append(Span::raw(".".repeat(usize::from(width.min(3)))))
  }

  fn fitting_length(
    graphemes: impl Iterator<Item = (Range<usize>, usize)>,
    width: u16,
  ) -> usize {
    let mut remaining = usize::from(width);

    graphemes
      .take_while(|(_, width)| {
        if *width > remaining {
          return false;
        }

        remaining -= width;

        true
      })
      .map(|(range, _)| range.len())
      .sum()
  }

  fn graphemes(
    text: &str,
  ) -> impl DoubleEndedIterator<Item = (Range<usize>, usize)> {
    text.grapheme_indices(true).map(|(index, grapheme)| {
      (
        index..index + grapheme.len(),
        UnicodeWidthStr::width(grapheme),
      )
    })
  }

  pub(crate) fn is_blank(&self) -> bool {
    self.spans.is_empty()
  }

  fn push(&mut self, span: Span) {
    if span.text.is_empty() {
      return;
    }

    match self.spans.last_mut() {
      Some(last) if last.style == span.style => last.text.push_str(&span.text),
      _ => self.spans.push(span),
    }
  }

  pub(crate) fn raw(text: impl Into<String>) -> Self {
    Self::from(Span::raw(text))
  }

  fn slice(&self, range: Range<usize>) -> Self {
    if range.is_empty() {
      return Self::blank();
    }

    let mut offset = 0;

    self
      .spans
      .iter()
      .filter_map(|span| {
        let (start, end) = (
          range.start.saturating_sub(offset),
          range.end.saturating_sub(offset).min(span.text.len()),
        );

        offset += span.text.len();

        if start < end {
          Some(Span::styled(&span.text[start..end], span.style))
        } else {
          None
        }
      })
      .collect()
  }

  pub(crate) fn styled(text: impl Into<String>, style: Style) -> Self {
    Self::from(Span::styled(text, style))
  }

  fn text(&self) -> Cow<'_, str> {
    match self.spans.as_slice() {
      [] => Cow::Borrowed(""),
      [span] => Cow::Borrowed(&span.text),
      spans => {
        Cow::Owned(spans.iter().map(|span| span.text.as_str()).collect())
      }
    }
  }

  pub(crate) fn width(&self) -> usize {
    Self::graphemes(&self.text()).map(|(_, width)| width).sum()
  }
}

impl Component for LineComponent {
  fn render(&self, width: u16) -> Vec<LineComponent> {
    if width == 0 || self.is_blank() {
      return vec![Self::blank()];
    }

    let max_width = usize::from(width);

    let text = self.text();

    let mut lines = Vec::new();
    let mut line = Self::blank();

    let (mut start, mut line_width) = (0, 0);

    for (range, grapheme_width) in Self::graphemes(&text) {
      let replace = grapheme_width > max_width;

      let grapheme_width = if replace { 1 } else { grapheme_width };

      if line_width + grapheme_width > max_width {
        lines.push(line.append(self.slice(start..range.start)));

        line = Self::blank();

        (start, line_width) = (range.start, 0);
      }

      if replace {
        line = line
          .append(self.slice(start..range.start))
          .append(Span::styled("�", self.slice(range.clone()).spans[0].style));

        start = range.end;
      }

      line_width += grapheme_width;
    }

    lines.push(line.append(self.slice(start..text.len())));

    lines
  }
}

impl Display for LineComponent {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    for span in &self.spans {
      if span.style == Style::None {
        write!(f, "{}", span.text)?;
      } else {
        write!(
          f,
          "{}{}{}",
          span.style.sequence(),
          span.text,
          Style::None.sequence()
        )?;
      }
    }

    Ok(())
  }
}

impl From<LineComponent> for Vec<Span> {
  fn from(line: LineComponent) -> Self {
    line.spans.into_vec()
  }
}

impl From<Span> for LineComponent {
  fn from(span: Span) -> Self {
    once(span).collect()
  }
}

impl From<Vec<Span>> for LineComponent {
  fn from(spans: Vec<Span>) -> Self {
    spans.into_iter().collect()
  }
}

impl<const N: usize> From<[Span; N]> for LineComponent {
  fn from(spans: [Span; N]) -> Self {
    spans.into_iter().collect()
  }
}

impl FromIterator<Span> for LineComponent {
  fn from_iter<T: IntoIterator<Item = Span>>(iter: T) -> Self {
    let mut line = Self::blank();

    for span in iter {
      line.push(span);
    }

    line
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn clips_at_grapheme_boundaries() {
    #[track_caller]
    fn case(text: &str, width: u16, prefix: &str, suffix: &str) {
      let line = LineComponent::raw(text);

      assert_eq!(line.clip_end(width), LineComponent::raw(prefix));
      assert_eq!(line.clip_start(width), LineComponent::raw(suffix));
    }

    case("", 1, "", "");
    case("foo", 0, "", "");
    case("foo", 2, "fo", "oo");
    case("foo", 3, "foo", "foo");
    case("👩‍💻foo👩‍💻", 1, "", "");
    case("👩‍💻foo👩‍💻", 2, "👩‍💻", "👩‍💻");
    case("\u{0301}foo", 3, "\u{0301}foo", "\u{0301}foo");
  }

  #[test]
  fn construction_normalizes_spans() {
    let line = LineComponent::from([
      Span::raw(""),
      Span::styled("foo", Style::Accent),
      Span::styled("", Style::Muted),
    ])
    .append(Span::styled("bar", Style::Accent))
    .append(Span::raw("baz"));

    assert_eq!(
      Vec::<Span>::from(line),
      [Span::styled("foobar", Style::Accent), Span::raw("baz")],
    );
  }

  #[test]
  fn displays_blank_line() {
    assert_eq!(LineComponent::blank().to_string(), "");
  }

  #[test]
  fn displays_escaped_control_characters_inside_style() {
    assert_eq!(
      LineComponent::from([Span::styled("\x1b[31mfoo", Style::Accent)])
        .to_string(),
      "\x1b[36;1m\\u{1b}[31mfoo\x1b[0m",
    );
  }

  #[test]
  fn displays_markdown_style_and_resets_attributes() {
    assert_eq!(
      LineComponent::from([
        Span::styled(
          "foo",
          Style::Markdown(
            anstyle::Style::new()
              .bold()
              .italic()
              .underline()
              .strikethrough()
          ),
        ),
        Span::raw("bar"),
      ])
      .to_string(),
      "\x1b[1m\x1b[3m\x1b[4m\x1b[9mfoo\x1b[0mbar",
    );
  }

  #[test]
  fn displays_mixed_styled_and_raw_text() {
    assert_eq!(
      LineComponent::from([
        Span::raw("a"),
        Span::styled("b", Style::Accent),
        Span::raw("c"),
      ])
      .to_string(),
      "a\x1b[36;1mb\x1b[0mc",
    );
  }

  #[test]
  fn ellipsizes_to_available_width() {
    #[track_caller]
    fn case(text: &str, width: u16, expected: &str) {
      assert_eq!(
        LineComponent::raw(text).ellipsize(width),
        LineComponent::raw(expected),
      );
    }

    case("foobarbaz", 0, "");
    case("foobarbaz", 1, ".");
    case("foobarbaz", 6, "foo...");
    case("foobarbaz", 9, "foobarbaz");
    case("👩‍💻foobar", 4, "...");
  }

  #[test]
  fn measures_display_width() {
    assert_eq!(LineComponent::raw("لا").width(), 2);
  }

  #[test]
  fn rendering_accounts_for_escaped_control_characters() {
    assert_eq!(
      LineComponent::raw("a\tb").render(3),
      [LineComponent::raw(r"a\t"), LineComponent::raw("b")],
    );
  }

  #[test]
  fn rendering_accounts_for_wide_characters() {
    #[track_caller]
    fn case(text: &str, width: u16, expected: &[&str]) {
      assert_eq!(
        LineComponent::raw(text).render(width),
        expected
          .iter()
          .copied()
          .map(LineComponent::raw)
          .collect::<Vec<_>>(),
      );
    }

    case("a界b", 1, &["a", "�", "b"]);
    case("a界b", 2, &["a", "界", "b"]);
    case("a界b", 3, &["a界", "b"]);
    case("a界b", 4, &["a界b"]);
    case("\u{17d8}foo", 2, &["�f", "oo"]);
  }

  #[test]
  fn rendering_keeps_graphemes_across_style_boundaries() {
    let emoji = LineComponent::styled("👩", Style::Accent)
      .append(Span::styled("\u{200d}💻", Style::Muted));

    assert_eq!(
      LineComponent::styled("foo", Style::Accent)
        .append(emoji.clone())
        .render(3),
      [LineComponent::styled("foo", Style::Accent), emoji],
    );
  }

  #[test]
  fn rendering_keeps_zero_width_combining_marks_with_line() {
    assert_eq!(
      LineComponent::raw("e\u{0301}x").render(1),
      [LineComponent::raw("e\u{0301}"), LineComponent::raw("x")],
    );
  }

  #[test]
  fn rendering_merges_adjacent_spans_with_same_style() {
    assert_eq!(
      LineComponent::from([
        Span::styled("foo", Style::Accent),
        Span::styled("bar", Style::Accent),
      ])
      .render(4),
      [
        LineComponent::from([Span::styled("foob", Style::Accent)]),
        LineComponent::from([Span::styled("ar", Style::Accent)]),
      ],
    );
  }

  #[test]
  fn rendering_preserves_plain_text_content() {
    let line = LineComponent::from([
      Span::raw("foo"),
      Span::styled("bar", Style::Accent),
      Span::raw("baz"),
    ]);

    let rendered_text = line
      .render(2)
      .into_iter()
      .flat_map(Vec::<Span>::from)
      .map(|span| span.text)
      .collect::<String>();

    assert_eq!(rendered_text, "foobarbaz");
  }

  #[test]
  fn rendering_preserves_style_boundaries() {
    assert_eq!(
      LineComponent::from([
        Span::styled("foo", Style::Accent),
        Span::styled("bar", Style::Muted),
      ])
      .render(6),
      [LineComponent::from([
        Span::styled("foo", Style::Accent),
        Span::styled("bar", Style::Muted),
      ])],
    );
  }

  #[test]
  fn renders_across_style_boundaries() {
    assert_eq!(
      LineComponent::from([
        Span::styled("foo", Style::Accent),
        Span::styled("bar", Style::Muted),
      ])
      .render(4),
      [
        LineComponent::from([
          Span::styled("foo", Style::Accent),
          Span::styled("b", Style::Muted),
        ]),
        LineComponent::from([Span::styled("ar", Style::Muted)]),
      ],
    );
  }

  #[test]
  fn renders_blank_line() {
    assert_eq!(LineComponent::blank().render(3), [LineComponent::blank()]);
  }

  #[test]
  fn renders_raw_text_at_various_widths() {
    #[track_caller]
    fn case(width: u16, expected: &[&str]) {
      assert_eq!(
        LineComponent::raw("foo").render(width),
        expected
          .iter()
          .copied()
          .map(LineComponent::raw)
          .collect::<Vec<_>>()
      );
    }

    case(0, &[""]);
    case(1, &["f", "o", "o"]);
    case(2, &["fo", "o"]);
    case(3, &["foo"]);
    case(4, &["foo"]);
  }

  #[test]
  fn renders_styled_text_at_width() {
    assert_eq!(
      LineComponent::from([Span::styled("foobar", Style::Accent)]).render(3),
      [
        LineComponent::from([Span::styled("foo", Style::Accent)]),
        LineComponent::from([Span::styled("bar", Style::Accent)]),
      ],
    );
  }

  #[test]
  fn renders_styled_text_that_exactly_fits_width() {
    assert_eq!(
      LineComponent::from([Span::styled("foo", Style::Accent)]).render(3),
      [LineComponent::from([Span::styled("foo", Style::Accent)])],
    );
  }
}
