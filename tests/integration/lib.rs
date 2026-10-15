use {
  anyhow::{Context, Error, bail, ensure},
  indoc::indoc,
  portable_pty::{CommandBuilder, PtySize, native_pty_system},
  rusqlite::Connection,
  std::{
    collections::BTreeMap,
    fs,
    io::{self, Read, Write},
    path::{Path, PathBuf},
    process::Stdio,
    str,
    sync::{
      Mutex,
      mpsc::{self, Receiver, RecvTimeoutError},
    },
    thread,
    time::{Duration, Instant},
  },
  tempfile::TempDir,
  tokio::{process::Command, runtime, time::timeout},
};

mod commands;
mod config;
mod conversation;
mod history;
mod input;
mod reasoning;
mod resume;
mod tools;

type Result<T = (), E = Error> = std::result::Result<T, E>;

const EXPECT_TIMEOUT: Duration = Duration::from_secs(5);
const READ_INTERVAL: Duration = Duration::from_millis(20);
const SCREEN_COLS: u16 = 80;
const SCREEN_PADDING: u16 = 2;
const SCREEN_ROWS: u16 = 24;
const SETTLE_INTERVAL: Duration = Duration::from_millis(200);
const STARTUP_TIMEOUT: Duration = Duration::from_secs(3);

static PTY_OPEN_LOCK: Mutex<()> = Mutex::new(());

#[derive(Clone, Copy, Debug)]
enum Key {
  AltEnter,
  CtrlC,
  CtrlJ,
  CtrlR,
  CtrlT,
  CtrlU,
  Down,
  Enter,
  Escape,
  Tab,
  Up,
}

impl Key {
  fn bytes(self) -> &'static [u8] {
    match self {
      Self::AltEnter => b"\x1b\r",
      Self::CtrlC => b"\x03",
      Self::CtrlJ => b"\n",
      Self::CtrlR => b"\x12",
      Self::CtrlT => b"\x14",
      Self::CtrlU => b"\x15",
      Self::Down => b"\x1b[B",
      Self::Enter => b"\r",
      Self::Escape => b"\x1b",
      Self::Tab => b"\t",
      Self::Up => b"\x1b[A",
    }
  }
}

#[derive(Clone, Copy)]
enum Mode {
  Interactive,
  Noninteractive,
}

#[derive(Debug)]
enum Step {
  ExpectExit(u32),
  ExpectScreenContains(String),
  ExpectScreenExcludes(String),
  Paste(String),
  Quit,
  Wait(Duration),
  Write(Vec<u8>),
}

struct Running {
  _master: Box<dyn portable_pty::MasterPty + Send>,
  child: Option<Box<dyn portable_pty::Child + Send + Sync>>,
  output: Receiver<Vec<u8>>,
  parser: vt100::Parser,
  writer: Box<dyn Write + Send>,
}

impl Running {
  fn drain_available(&mut self) {
    while let Ok(bytes) = self.output.try_recv() {
      self.parser.process(&bytes);
    }
  }

  fn expect_exit(&mut self, code: u32, timeout: Duration) -> Result {
    let status = self.wait_for_exit(timeout)?.with_context(|| {
      format!("timed out waiting for exit\n{}", self.screen())
    })?;

    ensure!(
      status.exit_code() == code,
      "expected exit code {code}, got {}\n{}",
      status.exit_code(),
      self.screen(),
    );

    Ok(())
  }

  fn expect_screen_contains(
    &mut self,
    text: &str,
    timeout: Duration,
  ) -> Result {
    self
      .wait_until(timeout, |running| running.screen().contains(text))
      .with_context(|| {
        format!(
          "timed out waiting for screen to contain `{text}`\n{}",
          self.screen()
        )
      })
  }

  fn expect_screen_excludes(
    &mut self,
    text: &str,
    timeout: Duration,
  ) -> Result {
    self
      .wait_until(timeout, |running| !running.screen().contains(text))
      .with_context(|| {
        format!(
          "timed out waiting for screen to exclude `{text}`\n{}",
          self.screen()
        )
      })
  }

  fn poll_exit(&mut self) -> Result<Option<portable_pty::ExitStatus>> {
    let status = match self.child.as_mut() {
      Some(child) => child.try_wait()?,
      None => None,
    };

    if status.is_some() {
      self.child.take();
    }

    Ok(status)
  }

  fn quit(&mut self) -> Result {
    self.write(b"\x03")?;

    if let Some(status) = self.wait_for_exit(Duration::from_millis(500))? {
      if status.success() {
        return Ok(());
      }

      bail!("unexpected exit status: {status}");
    }

    self.write(b"\x03")?;

    let status = self.wait_for_exit(EXPECT_TIMEOUT)?.with_context(|| {
      format!("timed out waiting for quit\n{}", self.screen())
    })?;

    if !status.success() {
      bail!("unexpected exit status: {status}");
    }

    Ok(())
  }

  fn read_thread(mut reader: Box<dyn Read + Send>) -> Receiver<Vec<u8>> {
    let (sender, receiver) = mpsc::channel();

    thread::spawn(move || {
      let mut buffer = [0; 8192];

      loop {
        match reader.read(&mut buffer) {
          Ok(0) => return,
          Ok(count) => {
            if sender.send(buffer[..count].into()).is_err() {
              return;
            }
          }
          Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
          Err(_) => return,
        }
      }
    });

    receiver
  }

  fn screen(&mut self) -> String {
    self.drain_available();

    let mut rows = self
      .parser
      .screen()
      .rows(SCREEN_PADDING, SCREEN_COLS - SCREEN_PADDING)
      .map(|row| row.trim_end().to_string())
      .collect::<Vec<_>>();

    while rows.last().is_some_and(String::is_empty) {
      rows.pop();
    }

    rows.join("\n")
  }

  fn spawn(test: &Test) -> Result<Self> {
    let pty_system = native_pty_system();

    let pair = {
      let _guard = PTY_OPEN_LOCK.lock().unwrap();

      pty_system.openpty(PtySize {
        cols: SCREEN_COLS,
        pixel_height: 0,
        pixel_width: 0,
        rows: SCREEN_ROWS,
      })?
    };

    let mut command = CommandBuilder::new(env!("CARGO_BIN_EXE_kotomori"));

    command.args(&test.arguments);

    command.cwd(test.cwd.as_deref().unwrap_or(test.tempdir.path()));

    command.env("KOTOMORI_CONFIG", test.tempdir.path().join("config.toml"));
    command.env("KOTOMORI_HOME", test.tempdir.path().join("kotomori-home"));
    command.env("RUST_BACKTRACE", "0");
    command.env("TERM", "xterm-256color");
    command.env("XDG_CONFIG_HOME", test.tempdir.path().join("xdg-config"));

    command.env_remove("KOTOMORI_DEV");

    for (key, value) in &test.env {
      command.env(key, value);
    }

    let child = pair.slave.spawn_command(command)?;

    let output = Self::read_thread(pair.master.try_clone_reader()?);

    let mut writer = pair.master.take_writer()?;

    if cfg!(windows) {
      writer.write_all(b"\x1b[1;1R")?;
      writer.flush()?;
    }

    Ok(Self {
      _master: pair.master,
      child: Some(child),
      output,
      parser: vt100::Parser::new(SCREEN_ROWS, SCREEN_COLS, 0),
      writer,
    })
  }

  fn wait_for_exit(
    &mut self,
    timeout: Duration,
  ) -> Result<Option<portable_pty::ExitStatus>> {
    let deadline = Instant::now() + timeout;

    loop {
      self.drain_available();

      if let Some(status) = self.poll_exit()? {
        return Ok(Some(status));
      }

      let Some(remaining) = deadline.checked_duration_since(Instant::now())
      else {
        return Ok(None);
      };

      match self.output.recv_timeout(remaining.min(READ_INTERVAL)) {
        Ok(bytes) => self.parser.process(&bytes),
        Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => {}
      }
    }
  }

  fn wait_until(
    &mut self,
    timeout: Duration,
    mut predicate: impl FnMut(&mut Self) -> bool,
  ) -> Result {
    let deadline = Instant::now() + timeout;

    loop {
      if predicate(self) {
        return Ok(());
      }

      if self.poll_exit()?.is_some() {
        if predicate(self) {
          return Ok(());
        }

        bail!("process exited before expectation was met");
      }

      let Some(remaining) = deadline.checked_duration_since(Instant::now())
      else {
        bail!("timed out");
      };

      match self.output.recv_timeout(remaining.min(READ_INTERVAL)) {
        Ok(bytes) => self.parser.process(&bytes),
        Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => {}
      }
    }
  }

  fn write(&mut self, bytes: &[u8]) -> Result {
    self.writer.write_all(bytes)?;
    self.writer.flush()?;
    Ok(())
  }
}

impl Drop for Running {
  fn drop(&mut self) {
    if let Some(mut child) = self.child.take() {
      let _ = child.kill();
      let _ = child.wait();
    }
  }
}

#[derive(Debug)]
struct Test {
  arguments: Vec<String>,
  cwd: Option<PathBuf>,
  env: Vec<(String, String)>,
  expected_stderr: Option<String>,
  expected_stdout: Option<String>,
  steps: Vec<Step>,
  tempdir: TempDir,
}

impl Test {
  fn argument(self, argument: &str) -> Self {
    self.arguments([argument])
  }

  fn arguments(
    mut self,
    arguments: impl IntoIterator<Item = impl Into<String>>,
  ) -> Self {
    self.arguments.extend(arguments.into_iter().map(Into::into));
    self
  }

  fn bytes(mut self, bytes: &[u8]) -> Self {
    self.steps.push(Step::Write(bytes.into()));
    self
  }

  fn config(self, config: &str) -> Self {
    let path = self.tempdir.path().join("config.toml");

    fs::write(&path, config).unwrap();

    self.env("KOTOMORI_CONFIG", path.to_str().unwrap())
  }

  fn cwd(mut self, cwd: &Path) -> Self {
    self.cwd = Some(cwd.into());
    self
  }

  fn env(mut self, key: &str, value: &str) -> Self {
    self.env.push((key.into(), value.into()));
    self
  }

  fn expect_exit(mut self, code: u32) -> Self {
    self.steps.push(Step::ExpectExit(code));
    self
  }

  fn expect_screen_contains(mut self, text: &str) -> Self {
    self.steps.push(Step::ExpectScreenContains(text.into()));
    self
  }

  fn expect_screen_excludes(mut self, text: &str) -> Self {
    self.steps.push(Step::ExpectScreenExcludes(text.into()));
    self
  }

  fn key(self, key: Key) -> Self {
    self.bytes(key.bytes())
  }

  fn keys(self, keys: impl IntoIterator<Item = Key>) -> Self {
    keys.into_iter().fold(self, Self::key)
  }

  fn model(self, model: &str) -> Self {
    self.arguments(["--model", model])
  }

  fn new() -> Self {
    let tempdir = tempfile::Builder::new()
      .prefix("kotomori-test")
      .tempdir()
      .unwrap();

    fs::write(
      tempdir.path().join("config.toml"),
      "default_provider = \"mock\"\ndefault_model = \"local\"\n",
    )
    .unwrap();

    Self {
      arguments: Vec::new(),
      cwd: None,
      env: Vec::new(),
      expected_stderr: None,
      expected_stdout: None,
      steps: Vec::new(),
      tempdir,
    }
  }

  fn paste(mut self, text: &str) -> Self {
    self.steps.push(Step::Paste(text.into()));
    self
  }

  fn quit(mut self) -> Self {
    self.steps.push(Step::Quit);
    self
  }

  fn run(self) -> Result {
    self.validate(Mode::Interactive)?;

    let mut running = Running::spawn(&self)?;

    running.expect_screen_contains("kotomori", STARTUP_TIMEOUT)?;

    for (index, step) in self.steps.into_iter().enumerate() {
      let context = format!("step {}: {step:?}", index + 1);

      let result = match step {
        Step::ExpectExit(code) => running.expect_exit(code, EXPECT_TIMEOUT),
        Step::ExpectScreenContains(text) => {
          running.expect_screen_contains(&text, EXPECT_TIMEOUT)
        }
        Step::ExpectScreenExcludes(text) => {
          running.expect_screen_excludes(&text, EXPECT_TIMEOUT)
        }
        Step::Paste(text) => {
          running.drain_available();

          let text = if running.parser.screen().bracketed_paste() {
            format!("\x1b[200~{text}\x1b[201~")
          } else {
            text
          };

          running.write(text.as_bytes())
        }
        Step::Quit => running.quit(),
        Step::Wait(duration) => {
          thread::sleep(duration);
          Ok(())
        }
        Step::Write(bytes) => running.write(&bytes),
      };

      result.context(context)?;
    }

    Ok(())
  }

  fn status(self, expected: i32) -> Result {
    self.validate(Mode::Noninteractive)?;

    let mut command = Command::new(env!("CARGO_BIN_EXE_kotomori"));

    command
      .args(&self.arguments)
      .current_dir(self.cwd.as_deref().unwrap_or(self.tempdir.path()))
      .env("KOTOMORI_CONFIG", self.tempdir.path().join("config.toml"))
      .env("KOTOMORI_HOME", self.tempdir.path().join("kotomori-home"))
      .env("RUST_BACKTRACE", "0")
      .env("XDG_CONFIG_HOME", self.tempdir.path().join("xdg-config"))
      .env_remove("KOTOMORI_DEV")
      .kill_on_drop(true)
      .stdin(Stdio::null());

    for (key, value) in &self.env {
      command.env(key, value);
    }

    let output = runtime::Builder::new_current_thread()
      .enable_all()
      .build()?
      .block_on(async { timeout(EXPECT_TIMEOUT, command.output()).await })
      .with_context(|| {
        format!(
          "timed out waiting for command with arguments {:?}",
          self.arguments,
        )
      })??;

    let stderr = str::from_utf8(&output.stderr).with_context(|| {
      format!("invalid UTF-8 in stderr: {}", output.stderr.escape_ascii())
    })?;

    let stdout = str::from_utf8(&output.stdout).with_context(|| {
      format!("invalid UTF-8 in stdout: {}", output.stdout.escape_ascii())
    })?;

    ensure!(
      output.status.code() == Some(expected),
      "expected exit code {expected}, got {:?}\n{stderr}",
      output.status.code(),
    );

    assert_eq!(stderr, self.expected_stderr.unwrap_or_default());

    assert_eq!(stdout, self.expected_stdout.unwrap_or_default());

    Ok(())
  }

  fn stderr(mut self, expected: &str) -> Self {
    self.expected_stderr = Some(expected.into());
    self
  }

  fn stdout(mut self, expected: &str) -> Self {
    self.expected_stdout = Some(expected.into());
    self
  }

  fn submit(self, text: &str) -> Self {
    self.type_text(text).key(Key::Enter)
  }

  fn success(self) -> Result {
    self.status(0)
  }

  fn type_text(self, text: &str) -> Self {
    self.bytes(text.as_bytes())
  }

  fn validate(&self, mode: Mode) -> Result {
    match mode {
      Mode::Interactive => {
        ensure!(
          self.expected_stderr.is_none(),
          "stderr expectations are not supported in an interactive run"
        );
        ensure!(
          self.expected_stdout.is_none(),
          "stdout expectations are not supported in an interactive run"
        );
      }
      Mode::Noninteractive => {
        if let Some(step) = self.steps.first() {
          bail!("step 1: {step:?} is not supported in a noninteractive run");
        }
      }
    }

    if let Some(index) = self
      .steps
      .iter()
      .position(|step| matches!(step, Step::ExpectExit(_) | Step::Quit))
    {
      for (index, step) in self.steps.iter().enumerate().skip(index + 1) {
        match step {
          Step::ExpectExit(_) | Step::Quit => bail!(
            "step {}: termination is not allowed after an exit expectation",
            index + 1
          ),
          Step::Paste(_) | Step::Write(_) => bail!(
            "step {}: input is not allowed after an exit expectation",
            index + 1
          ),
          Step::ExpectScreenContains(_)
          | Step::ExpectScreenExcludes(_)
          | Step::Wait(_) => {}
        }
      }
    }

    Ok(())
  }

  fn wait(mut self, duration: Duration) -> Self {
    self.steps.push(Step::Wait(duration));
    self
  }
}
