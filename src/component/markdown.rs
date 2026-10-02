use super::*;

#[derive(Debug)]
pub(crate) struct MarkdownComponent {
  text: String,
}

impl MarkdownComponent {
  pub(crate) fn new(text: &str) -> Self {
    Self { text: text.into() }
  }
}

impl Component for MarkdownComponent {
  fn render(&self, width: u16) -> Vec<LineComponent> {
    let text = tui_markdown::from_str_with_options(
      &self.text,
      &tui_markdown::Options::new(MarkdownStyleSheet).table_width(width),
    );

    text
      .lines
      .into_iter()
      .flat_map(|line| {
        let style = text.style.patch(line.style);

        LineComponent::from(line.spans.into_iter().fold(
          Vec::<Span>::new(),
          |mut spans, span| {
            let span =
              Span::styled(span.content, style.patch(span.style).into());

            if let Some(last) =
              spans.last_mut().filter(|last| last.style == span.style)
            {
              last.text.push_str(&span.text);
            } else {
              spans.push(span);
            }

            spans
          },
        ))
        .render(width)
      })
      .collect()
  }
}

#[derive(Clone)]
struct MarkdownStyleSheet;

impl tui_markdown::StyleSheet for MarkdownStyleSheet {
  fn code(&self) -> ratatui_core::style::Style {
    ratatui_core::style::Style::new().fg(ratatui_core::style::Color::Cyan)
  }

  fn code_block_fence(&self) -> &'static str {
    ""
  }

  fn heading_marker(&self, _level: u8) -> &'static str {
    ""
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn escapes_terminal_controls() {
    assert_eq!(
      MarkdownComponent::new("**foo\x1b[2J**").render(80),
      [LineComponent::from([Span::styled(
        r"foo\u{1b}[2J",
        Style::Markdown(anstyle::Style::new().bold()),
      )])],
    );
  }

  #[test]
  fn renders_code_and_headings_without_delimiters() {
    #[track_caller]
    fn case(text: &str, expected: &[&str]) {
      assert_eq!(
        MarkdownComponent::new(text)
          .render(80)
          .into_iter()
          .map(|line| line
            .spans
            .into_iter()
            .map(|span| span.text)
            .collect::<String>())
          .collect::<Vec<_>>(),
        expected,
      );
    }

    case("# foo", &["foo"]);
    case("```\nfoo\n```", &["foo"]);
    case("```\nfoo", &["foo"]);
    case("", &[]);
  }

  #[test]
  fn renders_styles() {
    assert_eq!(
      MarkdownComponent::new("# *foo*\n\n**bar** ~~baz~~ `qux`").render(80),
      [
        LineComponent::from([Span::styled(
          "foo",
          Style::Markdown(
            anstyle::Style::new()
              .bold()
              .italic()
              .underline()
              .bg_color(Some(anstyle::AnsiColor::Cyan.into()))
          ),
        )]),
        LineComponent::blank(),
        LineComponent::from([
          Span::styled("bar", Style::Markdown(anstyle::Style::new().bold())),
          Span::raw(" "),
          Span::styled(
            "baz",
            Style::Markdown(anstyle::Style::new().strikethrough())
          ),
          Span::raw(" "),
          Span::styled(
            "qux",
            Style::Markdown(
              anstyle::Style::new()
                .fg_color(Some(anstyle::AnsiColor::Cyan.into()))
            )
          ),
        ]),
      ],
    );
  }

  #[test]
  fn renders_tables() {
    #[track_caller]
    fn case(text: &str, width: u16, expected: &[&str]) {
      assert_eq!(
        MarkdownComponent::new(text)
          .render(width)
          .into_iter()
          .map(|line| line
            .spans
            .into_iter()
            .map(|span| span.text)
            .collect::<String>())
          .collect::<Vec<_>>(),
        expected,
      );
    }

    case(
      "| foo | bar | baz |\n| :--- | :---: | ---: |\n| x | y | z |",
      80,
      &[
        "┌─────┬─────┬─────┐",
        "│ foo │ bar │ baz │",
        "├─────┼─────┼─────┤",
        "│ x   │  y  │   z │",
        "└─────┴─────┴─────┘",
      ],
    );

    case(
      "| foo | bar |\n| --- | --- |\n| **foo baz** | qux |",
      15,
      &[
        "┌───────┬─────┐",
        "│ foo   │ bar │",
        "├───────┼─────┤",
        "│ foo   │ qux │",
        "│ baz   │     │",
        "└───────┴─────┘",
      ],
    );

    case(
      "| foo | bar |\n| --- | --- |\n| 界 | 👩‍💻 |",
      13,
      &[
        "┌─────┬─────┐",
        "│ foo │ bar │",
        "├─────┼─────┤",
        "│ 界  │ 👩‍💻  │",
        "└─────┴─────┘",
      ],
    );

    case(
      "| foo |\n| --- |\n| bar |",
      0,
      &["", "", "", "", "", "", "", "", ""],
    );
  }

  #[test]
  fn wraps_styled_text() {
    assert_eq!(
      MarkdownComponent::new("**foobar**").render(3),
      ["foo", "bar"].map(|text| LineComponent::from([Span::styled(
        text,
        Style::Markdown(anstyle::Style::new().bold()),
      )])),
    );
  }
}
