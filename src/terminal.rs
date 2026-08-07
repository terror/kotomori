use super::*;

#[derive(Debug)]
pub(crate) struct Terminal {
  pub(crate) stdout: BufWriter<Stdout>,
}

impl Terminal {
  pub(crate) fn new() -> Result<Self> {
    enable_raw_mode().context("failed to enable raw mode")?;

    let mut stdout = BufWriter::new(io::stdout());

    queue!(stdout, Hide).context("failed to hide cursor")?;

    Ok(Self { stdout })
  }
}

impl Drop for Terminal {
  fn drop(&mut self) {
    let _ = self.stdout.end_synchronized_update();

    let _ = execute!(self.stdout, MoveToColumn(0), MoveToNextLine(1), Show);

    let _ = self.stdout.flush();

    if let Err(error) = disable_raw_mode() {
      eprintln!("failed to restore terminal: {error}");
    }
  }
}
