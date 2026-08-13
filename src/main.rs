use {
  action::Action,
  agent::Agent,
  agent_activity::AgentActivity,
  agent_event::AgentEvent,
  agent_message_content::AgentMessageContent,
  anyhow::{Context, Error, bail},
  app::App,
  approval_policy::ApprovalPolicy,
  approval_request::ApprovalRequest,
  arguments::Arguments,
  async_trait::async_trait,
  changed_range::ChangedRange,
  channel::Channel,
  clap::{Args, Parser},
  command::Command,
  command_executor::CommandExecutor,
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
  database::Database,
  dimensions::Dimensions,
  directory_display::DirectoryDisplay,
  duration_ext::DurationExt,
  effect::Effect,
  event::Event,
  execution_limit::ExecutionLimit,
  frame::Frame,
  futures_util::StreamExt,
  home::home_dir,
  indoc::formatdoc,
  input_mode::InputMode,
  lexiclean::Lexiclean,
  loader::Loader,
  message::Message,
  model::Model,
  options::Options,
  patch::Patch,
  prompts::{COMPACTION_PROMPT, SYSTEM_PROMPT},
  provider::Provider,
  provider_content::ProviderContent,
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
  row_ext::RowExt,
  rusqlite::{Connection, TransactionBehavior, params},
  schemars::JsonSchema,
  screen::Screen,
  serde::{Deserialize, Serialize, de::DeserializeOwned},
  serde_json::Value,
  session::Session,
  settings::Settings,
  smallvec::SmallVec,
  span::Span,
  state::State,
  std::{
    backtrace::BacktraceStatus,
    collections::VecDeque,
    env,
    fmt::{self, Debug, Display, Formatter},
    fs,
    io::{self, BufWriter, Stdout, Write},
    iter::once,
    mem,
    path::{Path, PathBuf},
    process::{self, Stdio},
    str::{self, FromStr},
    sync::{Arc, LazyLock, Mutex, OnceLock},
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
  tool::ToolInvocationKind,
  tool_action_tense::ToolActionTense,
  tool_approval::ToolApproval,
  tool_call::ToolCall,
  tool_context::ToolContext,
  tool_invocation::ToolInvocation,
  tool_outcome::ToolOutcome,
  tool_result::ToolResult,
  transcript::Transcript,
  transcript_entry::TranscriptEntry,
  unicode_width::UnicodeWidthChar,
  user_message_content::UserMessageContent,
  write_ext::WriteExt,
};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

#[cfg(test)]
macro_rules! assert_matches {
  ($expression:expr, $( $pattern:pat_param )|+ $( if $guard:expr )? $(,)?) => {
    match $expression {
      $( $pattern )|+ $( if $guard )? => {}
      left => panic!(
        "assertion failed: (left ~= right)\n  left: `{:?}`\n right: `{}`",
        left,
        stringify!($($pattern)|+ $(if $guard)?)
      ),
    }
  }
}

#[cfg(test)]
use tool::CommandTool;

mod action;
mod agent;
mod agent_activity;
mod agent_event;
mod agent_message_content;
mod app;
mod approval_policy;
mod approval_request;
mod arguments;
mod changed_range;
mod channel;
mod command;
mod command_executor;
mod component;
mod composer;
mod config;
mod database;
mod dimensions;
mod directory_display;
mod duration_ext;
mod effect;
mod event;
mod execution_limit;
mod frame;
mod input_mode;
mod loader;
mod message;
mod model;
mod options;
mod patch;
mod prompts;
mod provider;
mod provider_content;
mod provider_sink;
mod raw_tool_call;
mod reasoning_buffer;
mod render_plan;
mod renderer;
mod request;
mod resume_picker;
mod resume_picker_action;
mod row_ext;
mod screen;
mod session;
mod settings;
mod span;
mod state;
mod str_ext;
mod style;
mod subcommand;
mod tool;
mod tool_action_tense;
mod tool_approval;
mod tool_call;
mod tool_context;
mod tool_invocation;
mod tool_outcome;
mod tool_result;
mod transcript;
mod transcript_entry;
mod user_message_content;
mod write_ext;

static FIRST_DRAW_STARTED_AT: OnceLock<Instant> = OnceLock::new();

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
