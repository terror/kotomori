use super::*;

#[derive(Debug)]
pub(crate) struct OutputPreviewComponent {
  pub(crate) limit: usize,
  pub(crate) output: String,
}

impl OutputPreviewComponent {
  fn preview(line: &str, width: usize) -> String {
    let line = Span::raw(line);
    let ellipsis = ".".repeat(width.min(3));

    let (mut preview, mut preview_width) = (String::new(), 0usize);

    for c in line.text.chars() {
      let char_width = UnicodeWidthChar::width(c).unwrap_or(0);

      if preview_width + char_width > width {
        while preview_width.saturating_add(ellipsis.len()) > width {
          let Some(c) = preview.pop() else {
            break;
          };

          preview_width = preview_width
            .saturating_sub(UnicodeWidthChar::width(c).unwrap_or(0));
        }

        preview.push_str(&ellipsis);

        return preview;
      }

      preview.push(c);
      preview_width += char_width;
    }

    preview
  }
}

impl Component for OutputPreviewComponent {
  fn render(&self, width: u16) -> Vec<LineComponent> {
    let output = self
      .output
      .lines()
      .filter(|line| !line.is_empty())
      .collect::<Vec<_>>();

    let mut lines = output
      .iter()
      .take(self.limit)
      .map(|line| LineComponent::raw(Self::preview(line, usize::from(width))))
      .collect::<Vec<_>>();

    let omitted = output.len().saturating_sub(self.limit);

    if omitted > 0 {
      lines.extend(
        LineComponent::from([Span::styled(
          format!(
            "... {omitted} more {}",
            if omitted == 1 { "line" } else { "lines" }
          ),
          Style::Muted,
        )])
        .render(width),
      );
    }

    lines
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn preview_fits_available_width() {
    #[track_caller]
    fn case(line: &str, width: usize, expected: &str) {
      assert_eq!(OutputPreviewComponent::preview(line, width), expected);
    }

    case("foobarbaz", 0, "");
    case("foobarbaz", 1, ".");
    case("foobarbaz", 2, "..");
    case("foobarbaz", 3, "...");
    case("foobarbaz", 6, "foo...");
    case("foobarbaz", 9, "foobarbaz");
    case("界foo", 4, "...");
    case("e\u{0301}foobar", 4, "e\u{0301}...");
    case("\tfoo", 4, r"\...");
  }
}
