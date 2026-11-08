#![allow(clippy::missing_errors_doc)]

use {
  ::rig::{client::env::EnvError, providers::openai::OpenAIConfig},
  anyhow::{Context, anyhow},
  futures_util::{StreamExt, future::BoxFuture},
  kotomori_core::{
    AgentMessage, AssistantContent, CallId, FinishReason, Message,
    MessageUpdate, Model, Provider, ProviderResponse, ProviderSink, Reasoning,
    ReasoningContent, Request, Result, Text, ToolFunction, ToolName,
    ToolRequest, Usage, UserMessageContent,
  },
  serde::{Deserialize, de::value},
  std::{env, path::PathBuf, sync::Arc},
};

#[cfg(test)]
use kotomori_core::{
  MessageBuffer, MessageDraft, ToolDefinition, ToolOutcome, ToolResult,
};

pub use {chatgpt::ChatGpt, copilot::GithubCopilot, rig::Rig};

mod chatgpt;
mod copilot;
mod rig;
