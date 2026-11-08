use {
  action::Action,
  agent_activity::AgentActivity,
  anyhow::{Context, Error, bail},
  app::App,
  arguments::Arguments,
  changed_range::ChangedRange,
  clap::{Args, Parser},
  clipboard::Clipboard,
  command::Command,
  command_arguments::CommandArguments,
  command_child::CommandChild,
  command_executor::CommandExecutor,
  component::{LineComponent, ViewComponent},
  composer::Composer,
  config::Config,
  crossterm::{
    cursor::{Hide, MoveDown, MoveTo, MoveToColumn, MoveUp},
    event::{
      self as crossterm_event, EnableBracketedPaste, Event as CrosstermEvent,
      KeyCode, KeyEvent, KeyEventKind, KeyModifiers,
    },
    queue,
    terminal::{
      self as crossterm_terminal, BeginSynchronizedUpdate, Clear, ClearType,
      EndSynchronizedUpdate, enable_raw_mode,
    },
  },
  database::Database,
  database_model::DatabaseModel,
  dimensions::Dimensions,
  directory_display::DirectoryDisplay,
  duration_ext::DurationExt,
  effect::Effect,
  event::Event,
  execution_limit::ExecutionLimit,
  frame::Frame,
  futures_util::{StreamExt, future::BoxFuture},
  history::History,
  history_navigation::HistoryNavigation,
  indoc::formatdoc,
  kotomori_core::{
    Agent, AgentConfig, AgentEvent, AgentMessage, ApprovalPolicy,
    ApprovalRequest, AssistantContent, CallId, Conversation, Message,
    MessageBuffer, MessageDraft, MessageUpdate, Model, Provider, ProviderSink,
    Reasoning, ReasoningContent, Request, Tool, ToolApproval, ToolCall,
    ToolDefinition, ToolFunction, ToolInvocation, ToolName, ToolOutcome,
    ToolRegistry, ToolRequest, ToolResult, UserMessageContent,
  },
  loader::Loader,
  options::Options,
  patch::Patch,
  path_ext::PathExt,
  process_wrap::tokio::{ChildWrapper, CommandWrap, KillOnDrop},
  ratatui_textarea::{CursorMove, DataCursor, Input, Key, TextArea},
  render_plan::RenderPlan,
  renderer::Renderer,
  resume_picker::ResumePicker,
  resume_picker_action::ResumePickerAction,
  run::Run,
  rusqlite::{
    Connection, TransactionBehavior, params,
    types::{FromSql, FromSqlError, FromSqlResult, ValueRef},
  },
  saved_session::SavedSession,
  schemars::JsonSchema,
  screen::Screen,
  scroll::Scroll,
  selection::Selection,
  serde::{Deserialize, Serialize},
  serde_json::Value,
  session::Session,
  session_summary::SessionSummary,
  settings::Settings,
  smallvec::SmallVec,
  span::Span,
  state::State,
  std::{
    backtrace::BacktraceStatus,
    borrow::Cow,
    collections::VecDeque,
    env,
    fmt::{self, Debug, Display, Formatter},
    fs,
    future::pending,
    io::{self, BufWriter, Stdout, Write},
    iter::once,
    ops::Range,
    path::{Component, Path, PathBuf},
    process::{self, Stdio},
    str,
    sync::{Arc, LazyLock, OnceLock},
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
    sync::mpsc::{self, UnboundedReceiver, UnboundedSender},
    time::{interval, sleep, timeout},
  },
  tool::CommandTool,
  tool_action_tense::ToolActionTense,
  transcript::Transcript,
  transcript_block::TranscriptBlock,
  transcript_entry::TranscriptEntry,
  unicode_segmentation::UnicodeSegmentation,
  unicode_width::UnicodeWidthStr,
  write_ext::WriteExt,
};

#[cfg(unix)]
use {process_wrap::tokio::ProcessGroup, std::os::unix::fs::PermissionsExt};

#[cfg(windows)]
use process_wrap::tokio::JobObject;

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
use temptree::temptree;

mod action;
mod agent_activity;
mod app;
mod arguments;
mod changed_range;
mod clipboard;
mod command;
mod command_arguments;
mod command_child;
mod command_executor;
mod component;
mod composer;
mod config;
mod database;
mod database_model;
mod dimensions;
mod directory_display;
mod duration_ext;
mod effect;
mod event;
mod execution_limit;
mod frame;
mod history;
mod history_navigation;
mod loader;
mod options;
mod patch;
mod path_ext;
mod provider;
mod render_plan;
mod renderer;
mod resume_picker;
mod resume_picker_action;
mod run;
mod saved_session;
mod screen;
mod scroll;
mod selection;
mod session;
mod session_summary;
mod settings;
mod span;
mod state;
mod str_ext;
mod style;
mod subcommand;
mod tool;
mod tool_action_tense;
mod transcript;
mod transcript_block;
mod transcript_entry;
mod write_ext;

static FIRST_DRAW_STARTED_AT: OnceLock<Instant> = OnceLock::new();

type AsyncCommand = tokio::process::Command;
type Result<T = (), E = Error> = std::result::Result<T, E>;

fn now() -> Result<u64> {
  Ok(
    SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .context("system clock is before the unix epoch")?
      .as_secs(),
  )
}

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
