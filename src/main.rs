use {
  action::Action,
  agent::Agent,
  anthropic_sdk as anthropic,
  anyhow::{Context, Error, bail},
  app::App,
  arguments::Arguments,
  async_openai as openai,
  async_trait::async_trait,
  clap::{Args, Parser},
  command::Command,
  command_invocation::CommandInvocation,
  component::Component,
  composer::Composer,
  crossterm::{
    cursor::{Hide, MoveDown, MoveToColumn, MoveToNextLine, MoveUp, Show},
    event::{
      self as crossterm_event, Event as CrosstermEvent, KeyCode, KeyEvent,
      KeyEventKind, KeyModifiers,
    },
    execute, queue,
    terminal::{
      self as crossterm_terminal, Clear, ClearType, disable_raw_mode,
      enable_raw_mode,
    },
  },
  effect::Effect,
  event::Event,
  footer::Footer,
  framed_lines::FramedLines,
  futures_util::StreamExt,
  header::Header,
  hint::Hint,
  line::Line,
  message::Message,
  model::Model,
  options::Options,
  provider::{Anthropic, Fake, Ollama, OpenAi, Provider},
  provider_sink::ProviderSink,
  ratatui_textarea::{CursorMove, Input, Key, TextArea},
  raw_tool_call::RawToolCall,
  refresh::Refresh,
  renderer::Renderer,
  request::Request,
  role::Role,
  serde_json::{Value, json},
  span::Span,
  state::State,
  std::{
    backtrace::BacktraceStatus,
    cmp::Ordering,
    collections::BTreeMap,
    env,
    fmt::{self, Debug, Display, Formatter},
    io::{self, Stdout, Write},
    iter::once,
    path::PathBuf,
    process,
    str::{self, FromStr},
    sync::Arc,
    thread,
    time::Duration,
  },
  strum::{EnumIter, IntoEnumIterator},
  style::Style,
  terminal::Terminal,
  tokio::{
    sync::mpsc::{self, UnboundedReceiver, UnboundedSender},
    time::{interval, sleep},
  },
  tool::RegisteredTool,
  tool_action_tense::ToolActionTense,
  tool_call_builder::ToolCallBuilder,
  tool_call_fragment::ToolCallFragment,
  tool_call_stream::ToolCallStream,
  tool_call_stream_event::ToolCallStreamEvent,
  tool_invocation::ToolInvocation,
  tool_invocation_kind::ToolInvocationKind,
  transcript::Transcript,
  unicode_width::UnicodeWidthChar,
  view::View,
};

mod action;
mod agent;
mod app;
mod arguments;
mod command;
mod command_invocation;
mod component;
mod composer;
mod effect;
mod event;
mod footer;
mod framed_lines;
mod header;
mod hint;
mod line;
mod message;
mod model;
mod options;
mod provider;
mod provider_sink;
mod raw_tool_call;
mod refresh;
mod renderer;
mod request;
mod role;
mod span;
mod state;
mod style;
mod terminal;
mod tool;
mod tool_action_tense;
mod tool_call_builder;
mod tool_call_fragment;
mod tool_call_stream;
mod tool_call_stream_event;
mod tool_invocation;
mod tool_invocation_kind;
mod transcript;
mod view;

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
