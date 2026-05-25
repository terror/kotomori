use super::*;

#[derive(Debug)]
pub(crate) struct LineStream<S> {
  buffer: Vec<u8>,
  stream: Pin<Box<S>>,
}

impl<S> LineStream<S>
where
  S: Stream,
{
  pub(crate) fn new(stream: S) -> Self {
    Self {
      buffer: Vec::new(),
      stream: Box::pin(stream),
    }
  }
}

impl<S, T, E> LineStream<S>
where
  E: Into<Error>,
  S: Stream<Item = std::result::Result<T, E>>,
  T: AsRef<[u8]>,
{
  pub(crate) async fn for_each(
    mut self,
    mut f: impl FnMut(&str) -> Result,
  ) -> Result {
    while let Some(chunk) = self.stream.next().await {
      let chunk = chunk.map_err(Into::into)?;

      self.buffer.extend_from_slice(chunk.as_ref());

      while let Some(index) = self.buffer.iter().position(|byte| *byte == b'\n')
      {
        let line = self.buffer.drain(..=index).collect::<Vec<_>>();

        f(str::from_utf8(&line[..line.len() - 1])?.trim())?;
      }
    }

    if !self.buffer.is_empty() {
      f(str::from_utf8(&self.buffer)?.trim())?;
    }

    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[tokio::test]
  async fn for_each() {
    let stream = futures_util::stream::iter([
      Ok::<&[u8], Error>("foo\nbar".as_bytes()),
      Ok::<&[u8], Error>("baz\nqux".as_bytes()),
    ]);
    let mut lines = Vec::new();

    LineStream::new(stream)
      .for_each(|line| {
        lines.push(line.to_string());

        Ok(())
      })
      .await
      .unwrap();

    assert_eq!(lines, ["foo", "barbaz", "qux"]);
  }
}
