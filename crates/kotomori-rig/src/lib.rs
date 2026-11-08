#![allow(clippy::missing_errors_doc)]

use {
  ::rig::{client::env::EnvError, providers::openai::OpenAIConfig},
  anyhow::Context,
  futures_util::{StreamExt, future::BoxFuture},
  kotomori_core::{
    AgentMessage, AssistantContent, Message, MessageUpdate, Model, Provider,
    ProviderSink, Request, Result, UserMessageContent,
  },
  std::{env, path::PathBuf, sync::Arc},
};

#[cfg(test)]
use kotomori_core::{
  CallId, MessageBuffer, MessageDraft, ToolDefinition, ToolName, ToolOutcome,
  ToolResult,
};

pub use {chatgpt::ChatGpt, copilot::GithubCopilot, rig::Rig};

mod chatgpt;
mod copilot;
mod rig;
