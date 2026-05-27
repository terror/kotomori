use {
  action::Action,
  agent::Agent,
  agent_activity::AgentActivity,
  anyhow::{Context, Error, bail},
  app::App,
  approval_prompt::ApprovalPrompt,
  approval_request::ApprovalRequest,
  arguments::Arguments,
  async_trait::async_trait,
  changed_range::ChangedRange,
  clap::{Args, Parser},
  command::Command,
  component::Component,
  composer::Composer,
  crossterm::{
    cursor::{
      Hide, MoveDown, MoveTo, MoveToColumn, MoveToNextLine, MoveUp, Show,
    },
    event::{
      self as crossterm_event, Event as CrosstermEvent, KeyCode, KeyEvent,
      KeyEventKind, KeyModifiers,
    },
    execute, queue,
    terminal::{
      self as crossterm_terminal, BeginSynchronizedUpdate, Clear, ClearType,
      EndSynchronizedUpdate, disable_raw_mode, enable_raw_mode,
    },
  },
  cursor::Cursor,
  diff::Diff,
  dimensions::Dimensions,
  duration_ext::DurationExt,
  effect::Effect,
  event::Event,
  execution_limit::ExecutionLimit,
  executor::Executor,
  footer::Footer,
  frame::Frame,
  framed_lines::FramedLines,
  futures_util::StreamExt,
  header::Header,
  hint::Hint,
  input_mode::InputMode,
  line::Line,
  loader::Loader,
  message::Message,
  message_kind::MessageKind,
  model::Model,
  options::Options,
  patch_plan::PatchPlan,
  presented_frame::PresentedFrame,
  provider::{Fake, Provider, Rig},
  provider_output::ProviderOutput,
  provider_sink::ProviderSink,
  ratatui_textarea::{CursorMove, Input, Key, TextArea},
  raw_tool_call::RawToolCall,
  render_plan::RenderPlan,
  render_planner::RenderPlanner,
  renderer::Renderer,
  request::Request,
  rig::{
    OneOrMany,
    client::CompletionClient,
    completion::{
      AssistantContent, CompletionModel, CompletionRequest,
      Message as RigMessage, ToolDefinition,
    },
    message::ToolCall,
    providers::{
      anthropic::{
        Client as AnthropicClient,
        completion::CompletionModel as AnthropicCompletionModel,
      },
      ollama::{
        Client as OllamaClient, CompletionModel as OllamaCompletionModel,
      },
      openai::{CompletionModel as OpenAiCompletionModel, CompletionsClient},
    },
    streaming::StreamedAssistantContent,
  },
  role::Role,
  schemars::JsonSchema,
  serde::{Deserialize, Serialize, de::DeserializeOwned},
  serde_json::Value,
  span::Span,
  state::State,
  std::{
    backtrace::BacktraceStatus,
    cmp::Ordering,
    env,
    fmt::{self, Debug, Display, Formatter},
    fs::{self, File},
    io::{self, BufRead, Read, Stdout, Write},
    iter::once,
    ops::RangeInclusive,
    path::{Path, PathBuf},
    process::{self, Stdio},
    str::{self, FromStr},
    sync::{Arc, LazyLock, Mutex},
    thread,
    time::Duration,
  },
  strum::{EnumIter, IntoEnumIterator},
  style::Style,
  terminal::Terminal,
  tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
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
  tools::{
    ApplyPatchTool, CommandTool, ListFilesTool, ReadFileTool, SearchFilesTool,
    WriteFileTool,
  },
  transcript::Transcript,
  transcript_entry::TranscriptEntry,
  transcript_tool_invocation::TranscriptToolInvocation,
  unicode_width::UnicodeWidthChar,
  view::View,
  viewport::Viewport,
  write_ext::WriteExt,
};

#[cfg(test)]
use serde_json::json;

mod action;
mod agent;
mod agent_activity;
mod anthropic {
  pub(crate) type Client = super::AnthropicClient;
  pub(crate) type CompletionModel = super::AnthropicCompletionModel;
}
mod app;
mod approval_prompt;
mod approval_request;
mod arguments;
mod changed_range;
mod command;
mod component;
mod composer;
mod cursor;
mod diff;
mod dimensions;
mod duration_ext;
mod effect;
mod event;
mod execution_limit;
mod executor;
mod footer;
mod frame;
mod framed_lines;
mod header;
mod hint;
mod input_mode;
mod line;
mod loader;
mod message;
mod message_kind;
mod model;
mod ollama {
  pub(crate) type Client = super::OllamaClient;
  pub(crate) type CompletionModel = super::OllamaCompletionModel;
}
mod openai {
  pub(crate) type CompletionModel = super::OpenAiCompletionModel;
  pub(crate) type CompletionsClient = super::CompletionsClient;
}
mod options;
mod patch_plan;
mod presented_frame;
mod provider;
mod provider_output;
mod provider_sink;
mod raw_tool_call;
mod render_plan;
mod render_planner;
mod renderer;
mod request;
mod role;
mod span;
mod state;
mod style;
mod terminal;
mod tool;
mod tool_action_tense;
mod tool_approval;
mod tool_invocation;
mod tool_invocation_kind;
mod tool_registry;
mod tool_result;
mod tool_spec;
mod tools;
mod transcript;
mod transcript_entry;
mod transcript_tool_invocation;
mod view;
mod viewport;
mod write_ext;

type Result<T = (), E = Error> = std::result::Result<T, E>;

#[tokio::main]
async fn main() {
  if let Err(error) = Arguments::parse().run().await {
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
