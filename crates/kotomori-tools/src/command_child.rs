use super::*;

#[derive(Debug)]
pub(crate) struct CommandChild {
  pub(crate) inner: Box<dyn ChildWrapper>,
}

impl CommandChild {
  pub(crate) fn spawn(command: AsyncCommand) -> io::Result<Self> {
    let mut command = CommandWrap::from(command);

    command.wrap(KillOnDrop);

    #[cfg(unix)]
    command.wrap(ProcessGroup::leader());

    #[cfg(windows)]
    command.wrap(JobObject);

    Ok(Self {
      inner: command.spawn()?,
    })
  }
}

impl Drop for CommandChild {
  fn drop(&mut self) {
    let _ = self.inner.start_kill();
  }
}
