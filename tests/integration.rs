use {
  anyhow::{Context, Error, bail, ensure},
  indoc::indoc,
  portable_pty::{CommandBuilder, PtySize, native_pty_system},
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

#[test]
fn approval_prompt_approves_command() -> Result {
  #[track_caller]
  fn case(key: &str) -> Result {
    Test::new()
      .model("mock:approval-required-command")
      .submit("foo")
      .expect_screen_contains(indoc! {
        "

        ? Approve echo bar?
        y approve · n/Esc deny

        mock · approval-required-command · \
        "
      })
      .type_text("x")
      .keys([Key::Tab, Key::Up, Key::Down, Key::Enter])
      .type_text(key)
      .expect_screen_contains(indoc! {
        "

        │ foo

        ● Ran echo bar
          │ bar

        done

        │

        mock · approval-required-command · \
        "
      })
      .quit()
      .run()
      .with_context(|| format!("approval key: {key:?}"))
  }

  case("y")?;
  case("Y")
}

#[test]
fn approval_prompt_denies_command() -> Result {
  #[track_caller]
  fn case(key: &[u8]) -> Result {
    Test::new()
      .model("mock:approval-required-command")
      .submit("foo")
      .expect_screen_contains("Approve echo bar?")
      .type_text("x")
      .keys([Key::Tab, Key::Up, Key::Down, Key::Enter])
      .bytes(key)
      .expect_screen_contains(indoc! {
        "

        │ foo

        ● Failed running echo bar
          │ permission denied

        done

        │

        mock · approval-required-command · \
        "
      })
      .quit()
      .run()
      .with_context(|| format!("denial key: {}", key.escape_ascii()))
  }

  case(b"n")?;
  case(b"N")?;
  case(Key::Escape.bytes())
}

#[test]
fn blank_submit_does_nothing() -> Result {
  #[track_caller]
  fn case(key: Key) -> Result {
    Test::new()
      .model("mock:local")
      .type_text("  ")
      .key(key)
      .type_text("foo")
      .expect_screen_contains(indoc! {
        "

        │   foo

        mock · local · \
        "
      })
      .key(Key::CtrlC)
      .expect_exit(0)
      .expect_screen_excludes("queued for mock:local:")
      .run()
      .with_context(|| format!("submission key: {key:?}"))
  }

  case(Key::Enter)?;
  case(Key::AltEnter)
}

#[cfg_attr(not(unix), ignore = "crossterm only emits paste events on Unix")]
#[test]
fn bracketed_paste_inserts_one_edit() -> Result {
  let input = indoc! {
    "

    │ foobar
    │ baz
    │ qux
    │ quux

    mock · local · \
    "
  };

  Test::new()
    .model("mock:local")
    .type_text("foo")
    .paste("bar\rbaz\r\nqux\nquux")
    .expect_screen_contains(input)
    .expect_screen_excludes("queued for mock:local:")
    .key(Key::CtrlU)
    .expect_screen_contains(indoc! {
      "

      │ foo

      mock · local · \
      "
    })
    .key(Key::CtrlR)
    .expect_screen_contains(input)
    .key(Key::Enter)
    .expect_screen_contains(indoc! {
      "

      │ foobar
      │ baz
      │ qux
      │ quux

      queued for mock:local: foobar baz qux quux

      │

      mock · local · \
      "
    })
    .quit()
    .run()
}

#[test]
fn command_clear_interrupts_active_agent() -> Result {
  #[track_caller]
  fn case(command: &str, key: Key) -> Result {
    Test::new()
      .model("mock:slow-streaming")
      .submit("foo")
      .expect_screen_contains("queued")
      .submit("bar")
      .expect_screen_contains(indoc! {
        "

        Queued
        │ bar

        │

        mock · slow-streaming · \
        "
      })
      .type_text(command)
      .key(key)
      .expect_screen_contains(indoc! {
        "

        Type a prompt. Press Ctrl-C to quit.

        │

        mock · slow-streaming · \
        "
      })
      .key(Key::Up)
      .type_text("baz")
      .expect_screen_contains(indoc! {
        "

        │ baz

        mock · slow-streaming · \
        "
      })
      .key(Key::Enter)
      .expect_screen_contains(indoc! {
        "

        Type a prompt. Press Ctrl-C to quit.

        │ baz

        queued for mock:slow-streaming: baz

        │

        mock · slow-streaming · \
        "
      })
      .quit()
      .run()
      .with_context(|| format!("command: {command:?}, submission key: {key:?}"))
  }

  case("/clear", Key::Enter)?;
  case("/cl", Key::Enter)?;
  case("/cl", Key::AltEnter)?;
  case("  /clear  ", Key::Enter)
}

#[test]
fn command_clears() -> Result {
  #[track_caller]
  fn case(command: &str) -> Result {
    Test::new()
      .model("mock:local")
      .submit("foo")
      .expect_screen_contains("queued for mock:local: foo")
      .submit(command)
      .expect_screen_excludes("│ foo")
      .expect_screen_excludes("queued for mock:local: foo")
      .expect_screen_contains(indoc! {
        "

        │

        mock · local · \
        "
      })
      .quit()
      .run()
      .with_context(|| format!("command: {command:?}"))
  }

  case("/")?;
  case("/c")?;
  case("/clear")
}

#[test]
fn command_completion_clears() -> Result {
  Test::new()
    .model("mock:local")
    .submit("foo")
    .expect_screen_contains("queued for mock:local: foo")
    .type_text("/")
    .expect_screen_contains("/clear")
    .key(Key::Down)
    .keys([Key::Up, Key::Tab])
    .key(Key::Enter)
    .expect_screen_excludes("queued for mock:local: foo")
    .run()
}

#[test]
fn command_completion_quits() -> Result {
  #[track_caller]
  fn case(keys: &[Key]) -> Result {
    Test::new()
      .model("mock:local")
      .type_text("/")
      .expect_screen_contains(indoc! {
        "

        │ /

        /clear  Clear the transcript
        /copy  Copy the last assistant response to the clipboard
        /fork  Branch the conversation to explore another approach
        /quit  Quit kotomori
        /rename  Rename the session

        mock · local · \
        "
      })
      .keys(keys.iter().copied())
      .key(Key::Tab)
      .expect_screen_contains(indoc! {
        "

        │ /quit

        /quit  Quit kotomori

        mock · local · \
        "
      })
      .key(Key::Enter)
      .expect_exit(0)
      .run()
      .with_context(|| format!("completion keys: {keys:?}"))
  }

  case(&[Key::Down, Key::Down, Key::Down])?;
  case(&[Key::Up, Key::Up])
}

#[test]
fn command_fork_persists_independent_sessions() -> Result {
  let state = tempfile::Builder::new()
    .prefix("kotomori-state")
    .tempdir()?;

  let workspace = tempfile::Builder::new()
    .prefix("kotomori-workspace")
    .tempdir()?;

  let state = state.path().to_str().unwrap();

  let original = indoc! {
    "

    Type a prompt. Press Ctrl-C to quit.

    │ foo

    queued for mock:local: foo

    │

    mock · local · \
    "
  };

  let forked = indoc! {
    "

    Type a prompt. Press Ctrl-C to quit.

    │ foo

    queued for mock:local: foo

    Forked conversation.

    Session renamed to 'bar'.

    │ baz

    queued for mock:local: baz

    │

    mock · local · \
    "
  };

  Test::new()
    .cwd(workspace.path())
    .env("KOTOMORI_HOME", state)
    .model("mock:local")
    .submit("foo")
    .expect_screen_contains(original)
    .submit("/fork")
    .expect_screen_contains("Forked conversation.")
    .submit("/rename bar")
    .expect_screen_contains("Session renamed to 'bar'.")
    .submit("baz")
    .expect_screen_contains(forked)
    .quit()
    .run()?;

  Test::new()
    .cwd(workspace.path())
    .env("KOTOMORI_HOME", state)
    .argument("resume")
    .submit("foo")
    .expect_screen_contains(original)
    .submit("qux")
    .expect_screen_contains("queued for mock:local: qux")
    .quit()
    .run()?;

  Test::new()
    .cwd(workspace.path())
    .env("KOTOMORI_HOME", state)
    .argument("resume")
    .submit("bar")
    .expect_screen_contains(forked)
    .quit()
    .run()
}

#[test]
fn command_quit_interrupts_active_agent_and_saves_partial_output() -> Result {
  #[track_caller]
  fn case(command: &str, key: Key) -> Result {
    let state = tempfile::Builder::new()
      .prefix("kotomori-state")
      .tempdir()?;

    let workspace = tempfile::Builder::new()
      .prefix("kotomori-workspace")
      .tempdir()?;

    let state = state.path().to_str().unwrap();

    let interrupted = indoc! {
      "

      ■ Conversation interrupted, tell the model what to do differently.

      │

      mock · slow-streaming · \
      "
    };

    Test::new()
      .cwd(workspace.path())
      .env("KOTOMORI_HOME", state)
      .model("mock:slow-streaming")
      .submit("foo")
      .expect_screen_contains("queued")
      .type_text(command)
      .key(key)
      .expect_screen_contains(interrupted)
      .expect_screen_contains("│ foo\n\nqueued")
      .expect_screen_excludes("queued for mock:slow-streaming: foo")
      .submit("/quit")
      .expect_exit(0)
      .run()
      .with_context(|| {
        format!("command: {command:?}, submission key: {key:?}")
      })?;

    Test::new()
      .cwd(workspace.path())
      .env("KOTOMORI_HOME", state)
      .arguments(["resume", "--last"])
      .expect_screen_contains(interrupted)
      .expect_screen_contains("│ foo\n\nqueued")
      .expect_screen_excludes("queued for mock:slow-streaming: foo")
      .quit()
      .run()
      .with_context(|| format!("command: {command:?}, submission key: {key:?}"))
  }

  case("/quit", Key::Enter)?;
  case("/q", Key::Enter)?;
  case("/q", Key::AltEnter)
}

#[test]
fn command_quits() -> Result {
  #[track_caller]
  fn case(command: &str) -> Result {
    Test::new()
      .config("")
      .model("mock:local")
      .submit(command)
      .expect_exit(0)
      .run()
  }

  case("/q")?;
  case("/quit")
}

#[test]
fn command_rename_persists() -> Result {
  let state = tempfile::Builder::new()
    .prefix("kotomori-state")
    .tempdir()?;

  let workspace = tempfile::Builder::new()
    .prefix("kotomori-workspace")
    .tempdir()?;

  let state = state.path().to_str().unwrap();

  Test::new()
    .cwd(workspace.path())
    .env("KOTOMORI_HOME", state)
    .model("mock:local")
    .submit("foo")
    .expect_screen_contains("queued for mock:local: foo")
    .submit("/rename")
    .expect_screen_contains("Usage: /rename <title>")
    .submit("  /rename  bar   baz  ")
    .expect_screen_contains("Session renamed to 'bar baz'.")
    .submit("qux")
    .expect_screen_contains("queued for mock:local: qux")
    .quit()
    .run()?;

  Test::new()
    .cwd(workspace.path())
    .env("KOTOMORI_HOME", state)
    .argument("resume")
    .expect_screen_contains("> bar baz  mock:local")
    .submit("bar baz")
    .expect_screen_contains("queued for mock:local: qux")
    .submit("quux")
    .expect_screen_contains("queued for mock:local: quux")
    .quit()
    .run()?;

  Test::new()
    .cwd(workspace.path())
    .env("KOTOMORI_HOME", state)
    .argument("resume")
    .expect_screen_contains("> bar baz  mock:local")
    .quit()
    .run()
}

#[test]
fn config_sets_default_model() -> Result {
  Test::new()
    .config(
      r#"
      default_provider = "mock"
      default_model = "bar"
      "#,
    )
    .submit("foo")
    .expect_screen_contains("queued for mock:bar: foo")
    .run()
}

#[test]
fn ctrl_c_interrupts_active_approval() -> Result {
  Test::new()
    .model("mock:approval-required-command")
    .submit("foo")
    .expect_screen_contains("Approve echo bar?")
    .key(Key::CtrlC)
    .expect_screen_contains("Conversation interrupted")
    .type_text("bar")
    .expect_screen_contains(indoc! {
      "

      │ foo

      ● Failed running echo bar
        │ interrupted

      ■ Conversation interrupted, tell the model what to do differently.

      │ bar

      mock · approval-required-command · \
      "
    })
    .key(Key::CtrlC)
    .expect_exit(0)
    .run()
}

#[test]
fn ctrl_c_quits_gracefully() -> Result {
  Test::new().quit().run()
}

#[test]
fn ctrl_t_toggles_reasoning() -> Result {
  let collapsed = indoc! {
    "

    │ foo

    Thinking...

    │

    mock · reasoning · \
    "
  };

  Test::new()
    .model("mock:reasoning")
    .submit("foo")
    .expect_screen_contains(collapsed)
    .key(Key::CtrlT)
    .expect_screen_contains(indoc! {
      "

      │ foo

      Thinking...
        │ bar
        │ baz
        │ qux

      │

      mock · reasoning · \
      "
    })
    .key(Key::CtrlT)
    .expect_screen_contains(collapsed)
    .run()
}

#[test]
fn dev_mode_controls_time_to_first_draw() -> Result {
  Test::new()
    .model("mock:local")
    .wait(SETTLE_INTERVAL)
    .expect_screen_excludes("first draw ")
    .run()?;

  Test::new()
    .env("KOTOMORI_DEV", "1")
    .model("mock:local")
    .wait(SETTLE_INTERVAL)
    .expect_screen_contains("first draw ")
    .run()
}

#[test]
fn directory_is_file() -> Result {
  let directory = tempfile::tempdir()?;

  let path = directory.path().join("foo");

  fs::write(&path, "bar")?;

  Test::new()
    .arguments(["--directory", path.to_str().unwrap()])
    .stderr(&format!(
      "error: `{}` is not a directory\n",
      path.canonicalize()?.display()
    ))
    .status(1)
}

#[test]
fn directory_is_missing() -> Result {
  let directory = tempfile::tempdir()?;

  let path = directory.path().join("foo");

  Test::new()
    .arguments(["--directory", path.to_str().unwrap()])
    .stderr(&format!(
      "error: failed to resolve directory `{}`\n\nbecause:\n- {}\n",
      path.display(),
      io::Error::from_raw_os_error(2),
    ))
    .status(1)
}

#[test]
fn immediate_submit_interrupts_active_agent_and_starts_new_run() -> Result {
  let queued = indoc! {
    "

    Queued
    │ baz

    │

    mock · slow-streaming · \
    "
  };

  Test::new()
    .model("mock:slow-streaming")
    .submit("foo")
    .expect_screen_contains("queued")
    .submit("baz")
    .expect_screen_contains(queued)
    .type_text("  bar  ")
    .key(Key::AltEnter)
    .expect_screen_contains("Conversation interrupted")
    .expect_screen_contains(queued)
    .expect_screen_contains(indoc! {
      "

      ■ Conversation interrupted, tell the model what to do differently.

      │   bar

      queued for mock:slow-streaming:   bar

      │ baz

      queued for mock:slow-streaming: baz

      │

      mock · slow-streaming · \
      "
    })
    .expect_screen_contains("│ foo\n\nqueued")
    .expect_screen_excludes("queued for mock:slow-streaming: foo")
    .key(Key::Up)
    .type_text("!")
    .expect_screen_contains(indoc! {
      "

      │   bar  !

      mock · slow-streaming · \
      "
    })
    .run()
}

#[test]
fn initial_prompt_submits() -> Result {
  Test::new()
    .model("mock:local")
    .arguments(["--prompt", "foo"])
    .key(Key::Enter)
    .expect_screen_contains("queued for mock:local: foo")
    .run()
}

#[test]
fn interrupt_active_agent() -> Result {
  #[track_caller]
  fn case(key: Key) -> Result {
    let test = Test::new()
      .model("mock:slow-streaming")
      .submit("foo")
      .expect_screen_contains("queued")
      .key(key)
      .expect_screen_contains("Conversation interrupted");

    let test = if matches!(key, Key::Escape) {
      test
        .key(Key::Escape)
        .wait(SETTLE_INTERVAL)
        .key(Key::Up)
        .expect_screen_contains(indoc! {
          "

          │ foo

          mock · slow-streaming · \
          "
        })
    } else {
      test
    };

    test
      .key(Key::CtrlC)
      .expect_exit(0)
      .run()
      .with_context(|| format!("interruption key: {key:?}"))
  }

  case(Key::CtrlC)?;
  case(Key::Escape)
}

#[test]
fn interrupt_advances_to_next_queued_submission() -> Result {
  Test::new()
    .model("mock:slow-streaming")
    .submit("foo")
    .expect_screen_contains("queued")
    .submit("bar")
    .expect_screen_contains(indoc! {
      "

      Queued
      │ bar

      │

      mock · slow-streaming · \
      "
    })
    .key(Key::Escape)
    .expect_screen_contains(indoc! {
      "

      ■ Conversation interrupted, tell the model what to do differently.

      │ bar

      queued for mock:slow-streaming: bar

      │

      mock · slow-streaming · \
      "
    })
    .expect_screen_excludes("queued for mock:slow-streaming: foo")
    .run()
}

#[test]
fn markdown_response() -> Result {
  Test::new()
    .model("mock:slow-streaming")
    .submit("**foo**")
    .expect_screen_contains("│ **foo**")
    .expect_screen_contains("queued for mock:slow-streaming: foo")
    .run()
}

#[test]
fn missing_config_defaults() -> Result {
  let directory = tempfile::tempdir()?;

  let path = directory.path().join("foo");

  Test::new()
    .env("KOTOMORI_CONFIG", path.to_str().unwrap())
    .submit("foo")
    .expect_screen_contains("queued for mock:local: foo")
    .quit()
    .run()?;

  assert!(path.is_file());

  assert_eq!(
    confy::load_path::<BTreeMap<String, String>>(&path)?,
    BTreeMap::new(),
  );

  Ok(())
}

#[test]
fn multiline_input() -> Result {
  let input = indoc! {
    "

    │ foo
    │ bar

    mock · local · \
    "
  };

  Test::new()
    .model("mock:local")
    .type_text("foo")
    .key(Key::CtrlJ)
    .type_text("bar")
    .expect_screen_contains(input)
    .key(Key::Enter)
    .expect_screen_contains(indoc! {
      "

      │ foo
      │ bar

      queued for mock:local: foo bar

      │

      mock · local · \
      "
    })
    .key(Key::Up)
    .expect_screen_contains(input)
    .run()
}

#[test]
fn prompt_history_edit_detaches_navigation() -> Result {
  Test::new()
    .model("mock:local")
    .submit("foo")
    .expect_screen_contains("queued for mock:local: foo")
    .key(Key::Up)
    .type_text("!")
    .key(Key::Down)
    .type_text("?")
    .expect_screen_contains(indoc! {
      "

      │ foo!?

      mock · local · \
      "
    })
    .key(Key::Up)
    .expect_screen_contains(indoc! {
      "

      │ foo

      mock · local · \
      "
    })
    .key(Key::Down)
    .expect_screen_contains(indoc! {
      "

      │ foo!?

      mock · local · \
      "
    })
    .quit()
    .run()
}

#[test]
fn prompt_history_is_cleared_by_clear_command() -> Result {
  Test::new()
    .model("mock:local")
    .submit("foo")
    .expect_screen_contains("queued for mock:local: foo")
    .submit("/clear")
    .key(Key::Up)
    .type_text("bar")
    .expect_screen_contains(indoc! {
      "

      │ bar

      mock · local · \
      "
    })
    .quit()
    .run()
}

#[test]
fn prompt_history_loads_session() -> Result {
  let state = tempfile::Builder::new()
    .prefix("kotomori-state")
    .tempdir()?;

  let workspace = tempfile::Builder::new()
    .prefix("kotomori-workspace")
    .tempdir()?;

  let state = state.path().to_str().unwrap();

  Test::new()
    .cwd(workspace.path())
    .env("KOTOMORI_HOME", state)
    .model("mock:local")
    .submit("foo")
    .expect_screen_contains("queued for mock:local: foo")
    .type_text("baz")
    .key(Key::CtrlJ)
    .submit("qux")
    .expect_screen_contains("queued for mock:local: baz qux")
    .quit()
    .run()?;

  Test::new()
    .cwd(workspace.path())
    .env("KOTOMORI_HOME", state)
    .arguments(["resume", "--last"])
    .key(Key::Up)
    .expect_screen_contains(indoc! {
      "

      │ baz
      │ qux

      mock · local · \
      "
    })
    .keys([Key::Up, Key::Up])
    .expect_screen_contains(indoc! {
      "

      │ foo

      mock · local · \
      "
    })
    .quit()
    .run()
}

#[test]
fn prompt_history_navigates_and_restores_draft() -> Result {
  Test::new()
    .model("mock:local")
    .submit("foo")
    .expect_screen_contains("queued for mock:local: foo")
    .submit("bar")
    .expect_screen_contains("queued for mock:local: bar")
    .type_text("baz")
    .key(Key::Up)
    .expect_screen_contains(indoc! {
      "

      │ bar

      mock · local · \
      "
    })
    .key(Key::Up)
    .expect_screen_contains(indoc! {
      "

      │ foo

      mock · local · \
      "
    })
    .key(Key::Down)
    .expect_screen_contains(indoc! {
      "

      │ bar

      mock · local · \
      "
    })
    .key(Key::Down)
    .expect_screen_contains(indoc! {
      "

      │ baz

      mock · local · \
      "
    })
    .quit()
    .run()
}

#[test]
fn prompt_history_preserves_multiline_navigation() -> Result {
  Test::new()
    .model("mock:local")
    .submit("foo")
    .expect_screen_contains("queued for mock:local: foo")
    .type_text("bar")
    .key(Key::CtrlJ)
    .type_text("baz")
    .key(Key::Up)
    .type_text("!")
    .expect_screen_contains(indoc! {
      "

      │ bar!
      │ baz

      mock · local · \
      "
    })
    .key(Key::Up)
    .expect_screen_contains(indoc! {
      "

      │ foo

      mock · local · \
      "
    })
    .key(Key::Down)
    .type_text("?")
    .expect_screen_contains(indoc! {
      "

      │ bar!
      │ baz?

      mock · local · \
      "
    })
    .quit()
    .run()
}

#[test]
fn prompt_round_trip() -> Result {
  Test::new()
    .model("mock:local")
    .submit("foo")
    .expect_screen_contains("foo")
    .expect_screen_contains("queued for mock:local: foo")
    .run()
}

#[test]
fn provider_error_recovers() -> Result {
  Test::new()
    .model("mock:error")
    .submit("foo")
    .expect_screen_contains("mock provider error")
    .submit("bar")
    .expect_screen_contains("queued for mock:error: bar")
    .run()
}

#[test]
fn provider_malformed_tool_arguments_recovers() -> Result {
  Test::new()
    .model("mock:malformed-tool-arguments")
    .submit("foo")
    .expect_screen_contains("failed to decode `command` arguments")
    .submit("bar")
    .expect_screen_contains("queued for mock:malformed-tool-arguments: bar")
    .run()
}

#[test]
fn provider_unknown_tool_recovers() -> Result {
  Test::new()
    .model("mock:unknown-tool")
    .submit("foo")
    .expect_screen_contains("unknown tool `unknown`")
    .submit("bar")
    .expect_screen_contains("queued for mock:unknown-tool: bar")
    .run()
}

#[test]
fn queued_submissions_run_in_order() -> Result {
  Test::new()
    .model("mock:slow-streaming")
    .submit("foo")
    .expect_screen_contains("queued")
    .type_text("bar")
    .expect_screen_contains(indoc! {
      "

      │ bar

      mock · slow-streaming · \
      "
    })
    .key(Key::Enter)
    .submit("baz")
    .expect_screen_contains(indoc! {
      "

      Queued
      │ bar

      Queued
      │ baz

      │

      mock · slow-streaming · \
      "
    })
    .expect_screen_contains(indoc! {
      "

      │ foo

      queued for mock:slow-streaming: foo

      │ bar

      queued for mock:slow-streaming: bar

      │ baz

      queued for mock:slow-streaming: baz

      │

      mock · slow-streaming · \
      "
    })
    .run()
}

#[test]
fn resume_filters_and_loads_session() -> Result {
  let state = tempfile::Builder::new()
    .prefix("kotomori-state")
    .tempdir()?;

  let workspace = tempfile::Builder::new()
    .prefix("kotomori-workspace")
    .tempdir()?;

  let other_workspace = tempfile::Builder::new()
    .prefix("kotomori-workspace")
    .tempdir()?;

  let state = state.path().to_str().unwrap();

  Test::new()
    .cwd(other_workspace.path())
    .env("KOTOMORI_HOME", state)
    .model("mock:local")
    .submit("qux")
    .expect_screen_contains("queued for mock:local: qux")
    .wait(SETTLE_INTERVAL)
    .run()?;

  Test::new()
    .env("KOTOMORI_HOME", state)
    .arguments(["--directory", workspace.path().to_str().unwrap()])
    .model("mock:local")
    .submit("bar")
    .expect_screen_contains("queued for mock:local: bar")
    .wait(SETTLE_INTERVAL)
    .run()?;

  Test::new()
    .cwd(workspace.path())
    .arguments(["--directory", "."])
    .env("KOTOMORI_HOME", state)
    .model("mock:local")
    .submit("foo")
    .expect_screen_contains("queued for mock:local: foo")
    .wait(SETTLE_INTERVAL)
    .run()?;

  Test::new()
    .env("KOTOMORI_HOME", state)
    .model("mock:other")
    .arguments(["resume", "--directory", workspace.path().to_str().unwrap()])
    .expect_screen_contains("foo")
    .expect_screen_contains("bar")
    .expect_screen_excludes("qux")
    .type_text("bar")
    .expect_screen_excludes("foo")
    .key(Key::Enter)
    .expect_screen_contains("queued for mock:local: bar")
    .submit("baz")
    .expect_screen_contains("queued for mock:local: baz")
    .wait(SETTLE_INTERVAL)
    .run()?;

  Test::new()
    .cwd(workspace.path())
    .env("KOTOMORI_HOME", state)
    .arguments(["resume", "--last"])
    .submit("latest")
    .expect_screen_contains("queued for mock:local: latest")
    .run()
}

#[test]
fn resume_loads_sessions_with_tools_and_interruptions() -> Result {
  let state = tempfile::Builder::new()
    .prefix("kotomori-state")
    .tempdir()?;

  let workspace = tempfile::Builder::new()
    .prefix("kotomori-workspace")
    .tempdir()?;

  let state = state.path().to_str().unwrap();

  Test::new()
    .cwd(workspace.path())
    .env("KOTOMORI_HOME", state)
    .model("mock:approval-required-command")
    .submit("approved tool result")
    .expect_screen_contains("Approve echo bar?")
    .type_text("y")
    .expect_screen_contains("Ran echo bar")
    .expect_screen_contains("done")
    .wait(SETTLE_INTERVAL)
    .run()?;

  Test::new()
    .cwd(workspace.path())
    .env("KOTOMORI_HOME", state)
    .model("mock:approval-required-command")
    .submit("denied tool result")
    .expect_screen_contains("Approve echo bar?")
    .type_text("n")
    .expect_screen_contains("Failed running echo bar")
    .expect_screen_contains("permission denied")
    .expect_screen_contains("done")
    .wait(SETTLE_INTERVAL)
    .run()?;

  Test::new()
    .cwd(workspace.path())
    .env("KOTOMORI_HOME", state)
    .model("mock:slow-streaming")
    .submit("interrupted response")
    .expect_screen_contains("queued")
    .key(Key::CtrlC)
    .expect_screen_contains("Conversation interrupted")
    .run()?;

  Test::new()
    .cwd(workspace.path())
    .env("KOTOMORI_HOME", state)
    .argument("resume")
    .submit("approved tool result")
    .expect_screen_contains("Ran echo bar")
    .expect_screen_contains("done")
    .run()?;

  Test::new()
    .cwd(workspace.path())
    .env("KOTOMORI_HOME", state)
    .argument("resume")
    .submit("denied tool result")
    .expect_screen_contains("Failed running echo bar")
    .expect_screen_contains("permission denied")
    .expect_screen_contains("done")
    .run()?;

  Test::new()
    .cwd(workspace.path())
    .env("KOTOMORI_HOME", state)
    .argument("resume")
    .submit("interrupted response")
    .expect_screen_contains("queued")
    .expect_screen_contains("Conversation interrupted")
    .run()
}

#[test]
fn resume_picker_cancels_with_escape_and_ctrl_c() -> Result {
  let state = tempfile::Builder::new()
    .prefix("kotomori-state")
    .tempdir()?;

  let workspace = tempfile::Builder::new()
    .prefix("kotomori-workspace")
    .tempdir()?;

  let state = state.path().to_str().unwrap();

  Test::new()
    .cwd(workspace.path())
    .env("KOTOMORI_HOME", state)
    .model("mock:local")
    .submit("foo")
    .expect_screen_contains("queued for mock:local: foo")
    .wait(SETTLE_INTERVAL)
    .run()?;

  Test::new()
    .cwd(workspace.path())
    .env("KOTOMORI_HOME", state)
    .argument("resume")
    .key(Key::Escape)
    .expect_exit(0)
    .run()?;

  Test::new()
    .cwd(workspace.path())
    .env("KOTOMORI_HOME", state)
    .argument("resume")
    .key(Key::CtrlC)
    .expect_exit(0)
    .run()
}

#[test]
fn resume_with_no_sessions_exits_successfully() -> Result {
  Test::new()
    .argument("resume")
    .stdout("No saved sessions.\n")
    .success()
}

#[test]
fn second_turn_conversation() -> Result {
  Test::new()
    .model("mock:local")
    .submit("foo")
    .expect_screen_contains("queued for mock:local: foo")
    .wait(SETTLE_INTERVAL)
    .submit("bar")
    .expect_screen_contains("queued for mock:local: bar")
    .run()
}

#[test]
fn submit_preserves_input_whitespace() -> Result {
  #[track_caller]
  fn case(key: Key) -> Result {
    Test::new()
      .model("mock:local")
      .type_text("  foo")
      .key(Key::CtrlJ)
      .type_text("  ")
      .key(key)
      .expect_screen_contains(indoc! {
        "

        │   foo
        │

        queued for mock:local:   foo

        │

        mock · local · \
        "
      })
      .key(Key::Up)
      .type_text("!")
      .expect_screen_contains(indoc! {
        "

        │   foo
        │   !

        mock · local · \
        "
      })
      .run()
      .with_context(|| format!("submission key: {key:?}"))
  }

  case(Key::Enter)?;
  case(Key::AltEnter)
}

#[test]
fn unknown_command() -> Result {
  #[track_caller]
  fn case(key: Key) -> Result {
    Test::new()
      .model("mock:local")
      .type_text("/foobar")
      .key(key)
      .expect_screen_contains(indoc! {
        "

        Type a prompt. Press Ctrl-C to quit.

        Unrecognized command '/foobar'. Type \"/\" for a list of supported commands.

        │

        mock · local · \
        "
      })
      .submit("foo")
      .expect_screen_contains(indoc! {
        "

        Type a prompt. Press Ctrl-C to quit.

        Unrecognized command '/foobar'. Type \"/\" for a list of supported commands.

        │ foo

        queued for mock:local: foo

        │

        mock · local · \
        "
      })
      .run()
      .with_context(|| format!("submission key: {key:?}"))
  }

  case(Key::Enter)?;
  case(Key::AltEnter)
}

#[test]
fn unknown_provider() -> Result {
  Test::new()
    .model("foo:bar")
    .stderr("error: no registered provider is named `foo`\n")
    .status(1)
}

#[test]
fn yolo_skips_approval() -> Result {
  Test::new()
    .model("mock:approval-required-command")
    .argument("--yolo")
    .submit("foo")
    .expect_screen_contains("Ran echo bar")
    .expect_screen_contains("bar")
    .expect_screen_contains("done")
    .expect_screen_excludes("Approve echo bar?")
    .run()
}
