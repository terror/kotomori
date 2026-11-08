#![allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]

use {
  anyhow::{Context, Error, bail, ensure},
  futures_util::future::BoxFuture,
  response_guard::ResponseGuard,
  run_context::RunContext,
  serde::{Deserialize, Serialize},
  serde_json::Value,
  std::{
    borrow::Cow,
    collections::{BTreeMap, BTreeSet, btree_map::Entry},
    fmt::{self, Debug, Display, Formatter},
    future::{Future, pending},
    mem,
    ops::AddAssign,
    str::FromStr,
    sync::{
      Arc, Mutex,
      atomic::{AtomicU64, Ordering},
    },
    thread,
    time::Instant,
  },
  tokio::{
    sync::{
      mpsc::{self, UnboundedReceiver, UnboundedSender},
      oneshot,
    },
    task,
    time::sleep_until,
  },
  tokio_util::sync::CancellationToken,
  tool_guard::ToolGuard,
};

#[cfg(test)]
use std::collections::VecDeque;

pub use {
  agent::Agent, agent_config::AgentConfig, agent_event::AgentEvent,
  agent_message::AgentMessage, agent_run::AgentRun,
  approval_policy::ApprovalPolicy, approval_request::ApprovalRequest,
  assistant_content::AssistantContent, call_id::CallId,
  context_strategy::ContextStrategy, conversation::Conversation,
  finish_reason::FinishReason, full_context::FullContext, message::Message,
  message_buffer::MessageBuffer, message_draft::MessageDraft,
  message_update::MessageUpdate, model::Model, provider::Provider,
  provider_response::ProviderResponse, provider_sink::ProviderSink,
  reasoning::Reasoning, reasoning_content::ReasoningContent, request::Request,
  request_context::RequestContext, response_id::ResponseId,
  response_outcome::ResponseOutcome, response_result::ResponseResult,
  run_entry::RunEntry, run_id::RunId, run_limit::RunLimit,
  run_outcome::RunOutcome, run_result::RunResult, run_state::RunState,
  text::Text, tool::Tool, tool_approval::ToolApproval, tool_call::ToolCall,
  tool_context::ToolContext, tool_definition::ToolDefinition,
  tool_function::ToolFunction, tool_name::ToolName, tool_outcome::ToolOutcome,
  tool_registry::ToolRegistry, tool_request::ToolRequest,
  tool_result::ToolResult, usage::Usage,
  user_message_content::UserMessageContent,
};

mod agent;
mod agent_config;
mod agent_event;
mod agent_message;
mod agent_run;
mod approval_policy;
mod approval_request;
mod assistant_content;
mod call_id;
mod context_strategy;
mod conversation;
mod finish_reason;
mod full_context;
mod message;
mod message_buffer;
mod message_draft;
mod message_update;
mod model;
mod provider;
mod provider_response;
mod provider_sink;
mod reasoning;
mod reasoning_content;
mod request;
mod request_context;
mod response_guard;
mod response_id;
mod response_outcome;
mod response_result;
mod run_context;
mod run_entry;
mod run_id;
mod run_limit;
mod run_outcome;
mod run_result;
mod run_state;
mod text;
mod tool;
mod tool_approval;
mod tool_call;
mod tool_context;
mod tool_definition;
mod tool_function;
mod tool_guard;
mod tool_name;
mod tool_outcome;
mod tool_registry;
mod tool_request;
mod tool_result;
mod usage;
mod user_message_content;

pub type Result<T = (), E = Error> = std::result::Result<T, E>;
