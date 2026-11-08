#![allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]

use {
  anyhow::{Context, Error, bail},
  futures_util::future::BoxFuture,
  run_context::RunContext,
  run_control::PendingApproval,
  serde::{Deserialize, Serialize},
  serde_json::Value,
  std::{
    collections::{BTreeMap, btree_map::Entry},
    fmt::{self, Debug, Display, Formatter},
    str::FromStr,
    sync::{
      Arc, Mutex, Weak,
      atomic::{AtomicU64, Ordering},
    },
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
use std::collections::VecDeque;

pub use {
  agent::Agent, agent_config::AgentConfig, agent_event::AgentEvent,
  agent_message::AgentMessage, approval_policy::ApprovalPolicy,
  approval_request::ApprovalRequest, approval_response::ApprovalResponse,
  assistant_content::AssistantContent, call_id::CallId,
  conversation::Conversation, message::Message, message_buffer::MessageBuffer,
  message_draft::MessageDraft, message_id::MessageId,
  message_update::MessageUpdate, model::Model, provider::Provider,
  provider_sink::ProviderSink, reasoning::Reasoning,
  reasoning_content::ReasoningContent, request::Request,
  run_control::RunControl, run_event::RunEvent, run_id::RunId, text::Text,
  tool::Tool, tool_approval::ToolApproval, tool_call::ToolCall,
  tool_definition::ToolDefinition, tool_function::ToolFunction,
  tool_id::ToolId, tool_invocation::ToolInvocation, tool_name::ToolName,
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
mod approval_response;
mod assistant_content;
mod call_id;
mod conversation;
mod message;
mod message_buffer;
mod message_draft;
mod message_id;
mod message_update;
mod model;
mod provider;
mod provider_sink;
mod reasoning;
mod reasoning_content;
mod request;
mod run_context;
mod run_control;
mod run_event;
mod run_id;
mod text;
mod tool;
mod tool_approval;
mod tool_call;
mod tool_definition;
mod tool_function;
mod tool_id;
mod tool_invocation;
mod tool_name;
mod tool_outcome;
mod tool_registry;
mod tool_request;
mod tool_result;
mod user_message_content;

pub type Result<T = (), E = Error> = std::result::Result<T, E>;
