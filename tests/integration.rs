#![cfg(not(windows))]

use {
  anyhow::{Context, Error, bail, ensure},
  portable_pty::{CommandBuilder, PtySize, native_pty_system},
  std::{
    fs,
    io::{self, Read, Write},
    path::{Path, PathBuf},
    sync::{
      Mutex,
      mpsc::{self, Receiver, RecvTimeoutError},
    },
    thread,
    time::{Duration, Instant},
  },
  tempfile::TempDir,
};

type Result<T = (), E = Error> = std::result::Result<T, E>;

const EXPECT_TIMEOUT: Duration = Duration::from_secs(5);
const READ_INTERVAL: Duration = Duration::from_millis(20);
const SCREEN_COLS: u16 = 80;
const SCREEN_ROWS: u16 = 24;
const SETTLE_INTERVAL: Duration = Duration::from_millis(200);
const STARTUP_TIMEOUT: Duration = Duration::from_secs(3);
static PTY_OPEN_LOCK: Mutex<()> = Mutex::new(());

#[derive(Debug)]
enum Step {
  ExpectExit(u32),
  ExpectScreenContains(String),
  ExpectScreenExcludes(String),
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
      .rows(0, SCREEN_COLS)
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
    let writer = pair.master.take_writer()?;

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
  steps: Vec<Step>,
  tempdir: TempDir,
}

impl Test {
  fn argument(mut self, argument: &str) -> Self {
    self.arguments.push(argument.into());
    self
  }

  fn config(self, config: &str) -> Self {
    let path = self.tempdir.path().join("config.toml");

    fs::write(&path, config).unwrap();

    self.env("KOTOMORI_CONFIG", path.to_str().unwrap())
  }

  fn ctrl_c(self) -> Self {
    self.write("\x03")
  }

  fn ctrl_j(self) -> Self {
    self.write("\n")
  }

  fn cwd(mut self, cwd: &Path) -> Self {
    self.cwd = Some(cwd.into());
    self
  }

  fn down(self) -> Self {
    self.write("\x1b[B")
  }

  fn enter(self) -> Self {
    self.write("\r")
  }

  fn env(mut self, key: &str, value: &str) -> Self {
    self.env.push((key.into(), value.into()));
    self
  }

  fn escape(self) -> Self {
    self.write("\x1b")
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

  fn new() -> Self {
    Self {
      arguments: Vec::new(),
      cwd: None,
      env: Vec::new(),
      steps: Vec::new(),
      tempdir: tempfile::Builder::new()
        .prefix("kotomori-test")
        .tempdir()
        .unwrap(),
    }
  }

  fn quit(mut self) -> Self {
    self.steps.push(Step::Quit);
    self
  }

  fn run(self) -> Result {
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

  fn tab(self) -> Self {
    self.write("\t")
  }

  fn type_text(mut self, text: &str) -> Self {
    self.steps.push(Step::Write(text.as_bytes().into()));
    self
  }

  fn wait(mut self, duration: Duration) -> Self {
    self.steps.push(Step::Wait(duration));
    self
  }

  fn write(mut self, bytes: &str) -> Self {
    self.steps.push(Step::Write(bytes.as_bytes().into()));
    self
  }
}

#[test]
fn approval_prompt_approves_command() -> Result {
  Test::new()
    .argument("--model")
    .argument("mock:approval-required-command")
    .type_text("foo")
    .enter()
    .expect_screen_contains("Approve echo bar?")
    .type_text("y")
    .expect_screen_contains("Ran echo bar")
    .expect_screen_contains("bar")
    .expect_screen_contains("done")
    .run()
}

#[test]
fn approval_prompt_denies_command() -> Result {
  Test::new()
    .argument("--model")
    .argument("mock:approval-required-command")
    .type_text("foo")
    .enter()
    .expect_screen_contains("Approve echo bar?")
    .type_text("n")
    .expect_screen_contains("Failed running echo bar")
    .expect_screen_contains("permission denied")
    .expect_screen_contains("done")
    .run()
}

#[test]
fn approval_prompt_denies_command_with_escape() -> Result {
  Test::new()
    .argument("--model")
    .argument("mock:approval-required-command")
    .type_text("foo")
    .enter()
    .expect_screen_contains("Approve echo bar?")
    .escape()
    .expect_screen_contains("Failed running echo bar")
    .expect_screen_contains("permission denied")
    .expect_screen_contains("done")
    .run()
}

#[test]
fn command_completion_clears() -> Result {
  Test::new()
    .argument("--model")
    .argument("mock:local")
    .type_text("foo")
    .enter()
    .expect_screen_contains("queued for mock:local: foo")
    .type_text("/")
    .expect_screen_contains("/clear")
    .tab()
    .enter()
    .expect_screen_excludes("queued for mock:local: foo")
    .run()
}

#[test]
fn command_completion_quits() -> Result {
  Test::new()
    .argument("--model")
    .argument("mock:local")
    .type_text("/")
    .expect_screen_contains("/clear")
    .down()
    .tab()
    .expect_screen_contains("/quit")
    .enter()
    .expect_exit(0)
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
    .type_text("foo")
    .enter()
    .expect_screen_contains("queued for mock:bar: foo")
    .run()
}

#[test]
fn ctrl_c_quits_gracefully() -> Result {
  Test::new().quit().run()
}

#[test]
fn dev_mode_controls_time_to_first_draw() -> Result {
  Test::new()
    .argument("--model")
    .argument("mock:local")
    .wait(SETTLE_INTERVAL)
    .expect_screen_excludes("first draw ")
    .run()?;

  Test::new()
    .env("KOTOMORI_DEV", "1")
    .argument("--model")
    .argument("mock:local")
    .wait(SETTLE_INTERVAL)
    .expect_screen_contains("first draw ")
    .run()
}

#[test]
fn initial_prompt_submits() -> Result {
  Test::new()
    .argument("--model")
    .argument("mock:local")
    .argument("--prompt")
    .argument("foo")
    .enter()
    .expect_screen_contains("queued for mock:local: foo")
    .run()
}

#[test]
fn interrupt_active_agent() -> Result {
  Test::new()
    .argument("--model")
    .argument("mock:slow-streaming")
    .type_text("foo")
    .enter()
    .ctrl_c()
    .expect_screen_contains("Conversation interrupted")
    .run()
}

#[test]
fn multiline_input() -> Result {
  Test::new()
    .argument("--model")
    .argument("mock:local")
    .type_text("foo")
    .ctrl_j()
    .type_text("bar")
    .enter()
    .expect_screen_contains("queued for mock:local: foo\n bar")
    .run()
}

#[test]
fn prompt_round_trip() -> Result {
  Test::new()
    .argument("--model")
    .argument("mock:local")
    .type_text("foo")
    .enter()
    .expect_screen_contains("foo")
    .expect_screen_contains("queued for mock:local: foo")
    .run()
}

#[test]
fn provider_error_recovers() -> Result {
  Test::new()
    .argument("--model")
    .argument("mock:error")
    .type_text("foo")
    .enter()
    .expect_screen_contains("mock provider error")
    .type_text("bar")
    .enter()
    .expect_screen_contains("queued for mock:error: bar")
    .run()
}

#[test]
fn provider_malformed_tool_arguments_recovers() -> Result {
  Test::new()
    .argument("--model")
    .argument("mock:malformed-tool-arguments")
    .type_text("foo")
    .enter()
    .expect_screen_contains("failed to decode `command` arguments")
    .type_text("bar")
    .enter()
    .expect_screen_contains("queued for mock:malformed-tool-arguments: bar")
    .run()
}

#[test]
fn provider_unknown_tool_recovers() -> Result {
  Test::new()
    .argument("--model")
    .argument("mock:unknown-tool")
    .type_text("foo")
    .enter()
    .expect_screen_contains("unknown tool `unknown`")
    .type_text("bar")
    .enter()
    .expect_screen_contains("queued for mock:unknown-tool: bar")
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
    .argument("--model")
    .argument("mock:local")
    .type_text("qux")
    .enter()
    .expect_screen_contains("queued for mock:local: qux")
    .wait(SETTLE_INTERVAL)
    .run()?;

  Test::new()
    .cwd(workspace.path())
    .env("KOTOMORI_HOME", state)
    .argument("--model")
    .argument("mock:local")
    .type_text("bar")
    .enter()
    .expect_screen_contains("queued for mock:local: bar")
    .wait(SETTLE_INTERVAL)
    .run()?;

  Test::new()
    .cwd(workspace.path())
    .env("KOTOMORI_HOME", state)
    .argument("--model")
    .argument("mock:local")
    .type_text("foo")
    .enter()
    .expect_screen_contains("queued for mock:local: foo")
    .wait(SETTLE_INTERVAL)
    .run()?;

  Test::new()
    .cwd(workspace.path())
    .env("KOTOMORI_HOME", state)
    .argument("--model")
    .argument("mock:other")
    .argument("resume")
    .expect_screen_contains("foo")
    .expect_screen_contains("bar")
    .expect_screen_excludes("qux")
    .type_text("bar")
    .expect_screen_excludes("foo")
    .enter()
    .expect_screen_contains("queued for mock:local: bar")
    .type_text("baz")
    .enter()
    .expect_screen_contains("queued for mock:local: baz")
    .run()
}

#[test]
fn second_turn_conversation() -> Result {
  Test::new()
    .argument("--model")
    .argument("mock:local")
    .type_text("foo")
    .enter()
    .expect_screen_contains("queued for mock:local: foo")
    .wait(SETTLE_INTERVAL)
    .type_text("bar")
    .enter()
    .expect_screen_contains("queued for mock:local: bar")
    .run()
}

#[test]
fn unknown_command() -> Result {
  Test::new()
    .argument("--model")
    .argument("mock:local")
    .type_text("/foobar")
    .enter()
    .expect_screen_contains("Unrecognized command '/foobar'")
    .run()
}

#[test]
fn yolo_skips_approval() -> Result {
  Test::new()
    .argument("--model")
    .argument("mock:approval-required-command")
    .argument("--yolo")
    .type_text("foo")
    .enter()
    .expect_screen_contains("Ran echo bar")
    .expect_screen_contains("bar")
    .expect_screen_contains("done")
    .expect_screen_excludes("Approve echo bar?")
    .run()
}
