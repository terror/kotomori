use {
  action::Action,
  agent::Agent,
  anyhow::{Context, Error},
  app::App,
  arguments::Arguments,
  clap::{Args, Parser},
  effect::Effect,
  event::Event,
  messages::Message,
  options::Options,
  ratatui::{
    DefaultTerminal, Frame,
    crossterm::event::{
      Event as CrosstermEvent, KeyCode, KeyEvent, KeyEventKind, KeyModifiers,
    },
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
  },
  role::Role,
  state::State,
  std::{backtrace::BacktraceStatus, process, thread, time::Duration},
  terminal::Terminal,
  tokio::{
    sync::mpsc::{self, UnboundedReceiver, UnboundedSender},
    time::sleep,
  },
};

mod action;
mod agent;
mod app;
mod arguments;
mod effect;
mod event;
mod messages;
mod options;
mod role;
mod state;
mod terminal;

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
