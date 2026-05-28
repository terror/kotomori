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
  component::{
    Component, FooterComponent, FramedLinesComponent, HeaderComponent,
    ViewComponent,
  },
  composer::Composer,
  config::Config,
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
  frame::Frame,
  futures_util::StreamExt,
  indoc::indoc,
  input_mode::InputMode,
  line::Line,
  loader::Loader,
  message::Message,
  model::Model,
  options::Options,
  patch_plan::PatchPlan,
  presented_frame::PresentedFrame,
  provider::{Mock, Provider, Rig},
  provider_output::ProviderOutput,
  provider_sink::ProviderSink,
  ratatui_textarea::{CursorMove, Input, Key, TextArea},
  raw_tool_call::RawToolCall,
  reasoning_buffer::ReasoningBuffer,
  render_plan::RenderPlan,
  render_planner::RenderPlanner,
  renderer::Renderer,
  request::Request,
  resume_picker::ResumePicker,
  resume_picker_action::ResumePickerAction,
  rig::{
    OneOrMany,
    client::CompletionClient,
    completion::{
      AssistantContent, CompletionModel, CompletionRequest,
      Message as RigMessage, ToolDefinition,
    },
    message::{Reasoning, ToolCall, ToolResultContent, UserContent},
    providers::{
      anthropic::{
        Client as AnthropicClient,
        completion::CompletionModel as AnthropicCompletionModel,
      },
      cohere::{
        Client as CohereClient, CompletionModel as CohereCompletionModel,
      },
      deepseek::{
        Client as DeepSeekClient, CompletionModel as DeepSeekCompletionModel,
      },
      gemini::{
        Client as GeminiClient, CompletionModel as GeminiCompletionModel,
      },
      groq::{Client as GroqClient, CompletionModel as GroqCompletionModel},
      mistral::{
        Client as MistralClient, CompletionModel as MistralCompletionModel,
      },
      moonshot::{
        Client as MoonshotClient, CompletionModel as MoonshotCompletionModel,
      },
      ollama::{
        Client as OllamaClient, CompletionModel as OllamaCompletionModel,
      },
      openai::{CompletionModel as OpenAiCompletionModel, CompletionsClient},
      openrouter::{
        Client as OpenRouterClient,
        CompletionModel as OpenRouterCompletionModel,
      },
      perplexity::{
        Client as PerplexityClient,
        CompletionModel as PerplexityCompletionModel,
      },
      together::{
        Client as TogetherClient, CompletionModel as TogetherCompletionModel,
      },
      xai::{Client as XaiClient, CompletionModel as XaiCompletionModel},
    },
    streaming::StreamedAssistantContent,
  },
  schemars::JsonSchema,
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
    cmp::{Ordering, Reverse},
    env,
    ffi::OsStr,
    fmt::{self, Debug, Display, Formatter},
    fs::{self, File},
    io::{self, BufRead, Read, Stdout, Write},
    iter::once,
    mem,
    ops::RangeInclusive,
    path::{Path, PathBuf},
    process::{self, Stdio},
    str::{self, FromStr},
    sync::{
      Arc, LazyLock, Mutex,
      atomic::{self, AtomicU64},
    },
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
  },
  strum::{EnumIter, IntoEnumIterator},
  style::Style,
  subcommand::Subcommand,
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
  user_message_content::UserMessageContent,
  viewport::Viewport,
  write_ext::WriteExt,
};

mod action;
mod agent;
mod agent_activity;
mod agent_message_content;
mod anthropic {
  pub(crate) type Client = super::AnthropicClient;
  pub(crate) type CompletionModel = super::AnthropicCompletionModel;
}
mod app;
mod approval_request;
mod arguments;
mod changed_range;
mod command;
mod config;
mod cohere {
  pub(crate) type Client = super::CohereClient;
  pub(crate) type CompletionModel = super::CohereCompletionModel;
}
mod component;
mod composer;
mod cursor;
mod deepseek {
  pub(crate) type Client = super::DeepSeekClient;
  pub(crate) type CompletionModel = super::DeepSeekCompletionModel;
}
mod diff;
mod dimensions;
mod duration_ext;
mod effect;
mod event;
mod execution_limit;
mod executor;
mod frame;
mod gemini {
  pub(crate) type Client = super::GeminiClient;
  pub(crate) type CompletionModel = super::GeminiCompletionModel;
}
mod groq {
  pub(crate) type Client = super::GroqClient;
  pub(crate) type CompletionModel = super::GroqCompletionModel;
}
mod input_mode;
mod line;
mod loader;
mod message;
mod mistral {
  pub(crate) type Client = super::MistralClient;
  pub(crate) type CompletionModel = super::MistralCompletionModel;
}
mod model;
mod moonshot {
  pub(crate) type Client = super::MoonshotClient;
  pub(crate) type CompletionModel = super::MoonshotCompletionModel;
}
mod ollama {
  pub(crate) type Client = super::OllamaClient;
  pub(crate) type CompletionModel = super::OllamaCompletionModel;
}
mod openai {
  pub(crate) type CompletionModel = super::OpenAiCompletionModel;
  pub(crate) type CompletionsClient = super::CompletionsClient;
}
mod openrouter {
  pub(crate) type Client = super::OpenRouterClient;
  pub(crate) type CompletionModel = super::OpenRouterCompletionModel;
}
mod options;
mod patch_plan;
mod perplexity {
  pub(crate) type Client = super::PerplexityClient;
  pub(crate) type CompletionModel = super::PerplexityCompletionModel;
}
mod presented_frame;
mod provider;
mod provider_output;
mod provider_sink;
mod raw_tool_call;
mod reasoning_buffer;
mod render_plan;
mod render_planner;
mod renderer;
mod request;
mod resume_picker;
mod resume_picker_action;
mod session;
mod session_file;
mod session_store;
mod session_summary;
mod settings;
mod span;
mod state;
mod style;
mod subcommand;
mod terminal;
mod together {
  pub(crate) type Client = super::TogetherClient;
  pub(crate) type CompletionModel = super::TogetherCompletionModel;
}
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
mod transcript_tool_invocation;
mod user_message_content;
mod viewport;
mod write_ext;
mod xai {
  pub(crate) type Client = super::XaiClient;
  pub(crate) type CompletionModel = super::XaiCompletionModel;
}

pub(crate) static SYSTEM_PROMPT: LazyLock<String> = LazyLock::new(|| {
  indoc! {
    "
    You are kotomori, a coding agent running on the user's machine.

    Work directly in the local repository. Inspect the code before changing it.
    Prefer small focused edits. Match the project's existing style.

    Use available tools to read, search, edit, and run automated checks.
    Preserve user changes. Avoid destructive commands unless explicitly requested.
    Report clearly what changed and what was verified.
    "
  }
  .trim_end()
  .to_string()
});

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
