#![allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]

use {
  anyhow::{Context, Error, bail},
  futures_util::future::BoxFuture,
  serde::{Deserialize, Serialize},
  serde_json::Value,
  std::{
    collections::{BTreeMap, btree_map::Entry},
    fmt::{self, Debug, Display, Formatter},
    iter::once,
    str::FromStr,
    sync::Arc,
  },
  tokio::{
    sync::{
      mpsc::{self, UnboundedReceiver, UnboundedSender},
      oneshot,
    },
    task,
  },
};

#[cfg(test)]
use std::{collections::VecDeque, sync::Mutex};

pub use {
  agent::Agent, agent_config::AgentConfig, agent_event::AgentEvent,
  agent_message::AgentMessage, approval_policy::ApprovalPolicy,
  approval_request::ApprovalRequest, assistant_content::AssistantContent,
  call_id::CallId, conversation::Conversation, message::Message,
  message_buffer::MessageBuffer, message_draft::MessageDraft,
  message_update::MessageUpdate, model::Model, provider::Provider,
  provider_sink::ProviderSink, reasoning::Reasoning,
  reasoning_content::ReasoningContent, request::Request, text::Text,
  tool::Tool, tool_approval::ToolApproval, tool_call::ToolCall,
  tool_definition::ToolDefinition, tool_function::ToolFunction,
  tool_invocation::ToolInvocation, tool_name::ToolName,
  tool_outcome::ToolOutcome, tool_registry::ToolRegistry,
  tool_request::ToolRequest, tool_result::ToolResult,
  user_message_content::UserMessageContent,
};

mod agent;
mod agent_config;
mod agent_event;
mod agent_message;
mod approval_policy;
mod approval_request;
mod assistant_content;
mod call_id;
mod conversation;
mod message;
mod message_buffer;
mod message_draft;
mod message_update;
mod model;
mod provider;
mod provider_sink;
mod reasoning;
mod reasoning_content;
mod request;
mod text;
mod tool;
mod tool_approval;
mod tool_call;
mod tool_definition;
mod tool_function;
mod tool_invocation;
mod tool_name;
mod tool_outcome;
mod tool_registry;
mod tool_request;
mod tool_result;
mod user_message_content;

pub type Result<T = (), E = Error> = std::result::Result<T, E>;
