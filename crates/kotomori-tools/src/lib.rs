use {
  command_child::CommandChild,
  kotomori_core::{
    ApprovalPolicy, Result, Tool, ToolCall, ToolDefinition, ToolOutcome,
    ToolResult,
  },
  process_wrap::tokio::{ChildWrapper, CommandWrap, KillOnDrop},
  schemars::JsonSchema,
  serde::{Deserialize, Serialize},
  serde_json::Value,
  std::{
    io,
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
  },
  tokio::{
    io::{AsyncRead, AsyncReadExt},
    time::timeout,
  },
};

#[cfg(unix)]
use process_wrap::tokio::ProcessGroup;

#[cfg(windows)]
use process_wrap::tokio::JobObject;

#[cfg(test)]
use {
  kotomori_core::{CallId, ToolFunction, ToolName, ToolRegistry, ToolRequest},
  std::{fs, sync::Arc},
};

pub use {
  command::CommandTool, command_arguments::CommandArguments,
  command_executor::CommandExecutor, execution_limit::ExecutionLimit,
};

mod command;
mod command_arguments;
mod command_child;
mod command_executor;
mod execution_limit;

type AsyncCommand = tokio::process::Command;
