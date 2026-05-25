use {
  action::Action,
  agent::Agent,
  anyhow::{Context, Error},
  app::App,
  arguments::Arguments,
  clap::{Args, Parser},
  command::Command,
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
  header::Header,
  hint::Hint,
  line::Line,
  message::Message,
  options::Options,
  refresh::Refresh,
  renderer::Renderer,
  role::Role,
  span::Span,
  state::State,
  std::{
    backtrace::BacktraceStatus,
    cmp::Ordering,
    fmt::{self, Display, Formatter},
    io::{self, Stdout, Write},
    iter::once,
    process, thread,
    time::Duration,
  },
  strum::{EnumIter, IntoEnumIterator},
  style::Style,
  terminal::Terminal,
  tokio::{
    sync::mpsc::{self, UnboundedReceiver, UnboundedSender},
    time::sleep,
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
mod component;
mod composer;
mod effect;
mod event;
mod header;
mod hint;
mod line;
mod message;
mod options;
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
