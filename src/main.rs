use {
  action::Action,
  agent::Agent,
  anyhow::{Context, Error, bail},
  app::App,
  arguments::Arguments,
  async_trait::async_trait,
  clap::{Args, Parser},
  command::Command,
  completion_request::CompletionRequest,
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
  futures_util::{Stream, StreamExt},
  header::Header,
  hint::Hint,
  line::Line,
  line_stream::LineStream,
  message::Message,
  model::Model,
  options::Options,
  provider::{Fake, Ollama, Provider},
  provider_sink::ProviderSink,
  ratatui_textarea::{CursorMove, Input, Key, TextArea},
  refresh::Refresh,
  renderer::Renderer,
  reqwest::Client,
  role::Role,
  serde::{Deserialize, Serialize},
  span::Span,
  state::State,
  std::{
    backtrace::BacktraceStatus,
    cmp::Ordering,
    env,
    fmt::{self, Display, Formatter},
    io::{self, Stdout, Write},
    iter::once,
    path::PathBuf,
    pin::Pin,
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
  transcript::Transcript,
  unicode_width::UnicodeWidthChar,
  view::View,
};

mod action;
mod agent;
mod app;
mod arguments;
mod command;
mod completion_request;
mod component;
mod composer;
mod effect;
mod event;
mod footer;
mod framed_lines;
mod header;
mod hint;
mod line;
mod line_stream;
mod message;
mod model;
mod options;
mod provider;
mod provider_sink;
mod refresh;
mod renderer;
mod role;
mod span;
mod state;
mod style;
mod terminal;
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
