#![allow(clippy::missing_errors_doc)]

use {
  ::rig::providers::openai::OpenAIConfig,
  anyhow::{Context, anyhow},
  futures_util::{StreamExt, future::BoxFuture},
  kotomori_core::{
    AgentMessage, AssistantContent, CallId, FinishReason, Message,
    MessageUpdate, Provider, ProviderResponse, ProviderSink, Reasoning,
    ReasoningContent, Request, Result, Text, ToolChoice, ToolFunction,
    ToolName, ToolRequest, Usage, UserMessageContent,
  },
  serde::{Deserialize, de::value},
  std::sync::Arc,
};

#[cfg(test)]
use kotomori_core::{
  GenerationOptions, MessageBuffer, MessageDraft, Model, ToolDefinition,
  ToolOutcome, ToolResult,
};

pub use {chatgpt::ChatGpt, copilot::GithubCopilot, rig::Rig};

mod chatgpt;
mod copilot;
mod rig;
