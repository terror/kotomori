use {
  action::Action,
  agent::Agent,
  agent_activity::AgentActivity,
  agent_message_content::AgentMessageContent,
  anyhow::{Context, Error, bail},
  app::App,
  approval_request::ApprovalRequest,
  arguments::Arguments,
  async_trait::async_trait,
  changed_range::ChangedRange,
  clap::{Args, Parser},
  command::Command,
  component::{Component, ViewComponent},
  composer::Composer,
  config::Config,
  crossterm::{
    cursor::{Hide, MoveDown, MoveTo, MoveToColumn, MoveUp},
    event::{
      self as crossterm_event, Event as CrosstermEvent, KeyCode, KeyEvent,
      KeyEventKind, KeyModifiers,
    },
    queue,
    terminal::{
      self as crossterm_terminal, BeginSynchronizedUpdate, Clear, ClearType,
      EndSynchronizedUpdate, enable_raw_mode,
    },
  },
  diff::Diff,
  dimensions::Dimensions,
  directory_display::DirectoryDisplay,
  duration_ext::DurationExt,
  effect::Effect,
  event::Event,
  execution_limit::ExecutionLimit,
  executor::Executor,
  frame::Frame,
  futures_util::StreamExt,
  home::home_dir,
  indoc::indoc,
  input_mode::InputMode,
  lexiclean::Lexiclean,
  loader::Loader,
  message::Message,
  model::Model,
  options::Options,
  presented_frame::PresentedFrame,
  provider::Provider,
  provider_sink::ProviderSink,
  ratatui_textarea::{CursorMove, DataCursor, Input, Key, TextArea},
  raw_tool_call::RawToolCall,
  reasoning_buffer::ReasoningBuffer,
  render_plan::RenderPlan,
  renderer::Renderer,
  request::Request,
  resume_picker::ResumePicker,
  resume_picker_action::ResumePickerAction,
  rig::{
    OneOrMany,
    completion::{
      AssistantContent, CompletionRequest, Message as RigMessage,
      ToolDefinition,
    },
    message::{Reasoning, ToolResultContent, UserContent},
  },
  schemars::JsonSchema,
  screen::Screen,
  serde::{Deserialize, Serialize, de::DeserializeOwned},
  serde_json::Value,
  session::Session,
  session_file::SessionFile,
  session_store::SessionStore,
  session_summary::SessionSummary,
  settings::Settings,
  smallvec::SmallVec,
  span::Span,
  state::State,
  std::{
    backtrace::BacktraceStatus,
    cmp::Reverse,
    env,
    ffi::OsStr,
    fmt::{self, Debug, Display, Formatter},
    fs,
    io::{self, BufWriter, Stdout, Write},
    iter::once,
    mem,
    path::{Path, PathBuf},
    process::{self, Stdio},
    str::{self, FromStr},
    sync::{
      Arc, LazyLock, Mutex, OnceLock,
      atomic::{self, AtomicU64},
    },
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
  },
  str_ext::StrExt,
  strum::{EnumIter, IntoEnumIterator},
  style::Style,
  subcommand::Subcommand,
  tokio::{
    io::{AsyncRead, AsyncReadExt},
    runtime::Runtime,
    sync::{
      mpsc::{self, UnboundedReceiver, UnboundedSender},
      oneshot,
    },
    task,
    time::{interval, sleep, timeout},
  },
  tool::Tool,
  tool_action_tense::ToolActionTense,
  tool_approval::ToolApproval,
  tool_invocation::ToolInvocation,
  tool_invocation_kind::ToolInvocationKind,
  tool_registry::ToolRegistry,
  tool_result::ToolResult,
  tool_spec::ToolSpec,
  tools::CommandTool,
  transcript::Transcript,
  transcript_entry::TranscriptEntry,
  unicode_width::UnicodeWidthChar,
  user_message_content::UserMessageContent,
  viewport::Viewport,
  write_ext::WriteExt,
};

mod action;
mod agent;
mod agent_activity;
mod agent_message_content;
mod app;
mod approval_request;
mod arguments;
mod changed_range;
mod command;
mod component;
mod composer;
mod config;
mod diff;
mod dimensions;
mod directory_display;
mod duration_ext;
mod effect;
mod event;
mod execution_limit;
mod executor;
mod frame;
mod input_mode;
mod loader;
mod message;
mod model;
mod options;
mod presented_frame;
mod provider;
mod provider_sink;
mod raw_tool_call;
mod reasoning_buffer;
mod render_plan;
mod renderer;
mod request;
mod resume_picker;
mod resume_picker_action;
mod screen;
mod session;
mod session_file;
mod session_store;
mod session_summary;
mod settings;
mod span;
mod state;
mod str_ext;
mod style;
mod subcommand;
#[macro_use]
mod tools;
mod tool;
mod tool_action_tense;
mod tool_approval;
mod tool_invocation;
mod tool_invocation_kind;
mod tool_registry;
mod tool_result;
mod tool_spec;
mod transcript;
mod transcript_entry;
mod user_message_content;
mod viewport;
mod write_ext;

static FIRST_DRAW_STARTED_AT: OnceLock<Instant> = OnceLock::new();

pub(crate) static SYSTEM_PROMPT: LazyLock<String> = LazyLock::new(|| {
  indoc! {
    "
    You are kotomori, a coding agent running on the user's machine.

    Work directly in the local repository. Inspect the code before changing it.
    Prefer small focused edits. Match the project's existing style.

    Use the command tool to read, search, edit, and run automated checks.
    Preserve user changes. Avoid destructive commands unless explicitly requested.
    Report clearly what changed and what was verified.
    "
  }
  .trim_end()
  .to_string()
});

type AsyncCommand = tokio::process::Command;
type OutputTask = task::JoinHandle<io::Result<String>>;
type Result<T = (), E = Error> = std::result::Result<T, E>;

fn main() {
  let first_draw_started_at = Instant::now();

  if env::var_os("KOTOMORI_DEV").is_some() {
    FIRST_DRAW_STARTED_AT.get_or_init(|| first_draw_started_at);
  }

  let result = Runtime::new()
    .context("failed to initialize async runtime")
    .and_then(|runtime| runtime.block_on(Arguments::parse().run()));

  if let Err(error) = result {
    eprintln!("error: {error}");

    for (i, error) in error.chain().skip(1).enumerate() {
      if i == 0 {
        eprintln!();
        eprintln!("because:");
      }

      eprintln!("- {error}");
    }

    let backtrace = error.backtrace();

    if backtrace.status() == BacktraceStatus::Captured {
      eprintln!("backtrace:");
      eprintln!("{backtrace}");
    }

    process::exit(1);
  }
}
