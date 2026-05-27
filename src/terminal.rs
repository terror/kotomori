use super::*;

#[derive(Debug)]
pub(crate) struct Terminal {
  stdout: Stdout,
}

impl Terminal {
  pub(crate) fn new() -> Result<Self> {
    enable_raw_mode().context("failed to enable raw mode")?;

    let mut stdout = io::stdout();

    execute!(stdout, Hide).context("failed to hide cursor")?;

    Ok(Self { stdout })
  }

  pub(crate) fn stdout_mut(&mut self) -> &mut Stdout {
    &mut self.stdout
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
