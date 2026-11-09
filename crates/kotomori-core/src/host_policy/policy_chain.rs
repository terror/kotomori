use super::*;

#[derive(Clone, Debug, Default)]
pub struct PolicyChain {
  policies: Vec<Arc<dyn HostPolicy>>,
}

impl PolicyChain {
  #[must_use]
  pub fn new(policies: Vec<Arc<dyn HostPolicy>>) -> Self {
    Self { policies }
  }
}

impl HostPolicy for PolicyChain {
  fn evaluate<'a>(
    &'a self,
    context: &'a RunContext,
    invocation: &'a ToolRequest,
    approval: ApprovalPolicy,
  ) -> BoxFuture<'a, Result<PolicyDecision>> {
    Box::pin(async move {
      let mut decision = PolicyDecision::Allow;

      for policy in &self.policies {
        match policy.evaluate(context, invocation, approval).await? {
          PolicyDecision::Allow => {}
          PolicyDecision::Ask => decision = PolicyDecision::Ask,
          decision @ PolicyDecision::Deny { .. } => return Ok(decision),
        }
      }

      Ok(decision)
    })
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[derive(Debug)]
  struct TestPolicy {
    calls: Arc<AtomicU64>,
    decision: Result<PolicyDecision, &'static str>,
  }

  impl HostPolicy for TestPolicy {
    fn evaluate<'a>(
      &'a self,
      _context: &'a RunContext,
      _invocation: &'a ToolRequest,
      _approval: ApprovalPolicy,
    ) -> BoxFuture<'a, Result<PolicyDecision>> {
      Box::pin(async move {
        self.calls.fetch_add(1, Ordering::Relaxed);
        self.decision.clone().map_err(Error::msg)
      })
    }
  }

  #[tokio::test]
  async fn policies_preserve_restrictions_and_stop_on_denial_or_error() {
    async fn case(
      decisions: Vec<Result<PolicyDecision, &'static str>>,
      expected: Result<PolicyDecision, &'static str>,
      calls: u64,
    ) {
      let count = Arc::new(AtomicU64::new(0));

      let policy = PolicyChain::new(
        decisions
          .into_iter()
          .map(|decision| {
            Arc::new(TestPolicy {
              calls: count.clone(),
              decision,
            }) as Arc<dyn HostPolicy>
          })
          .collect(),
      );

      let (sender, _) = mpsc::unbounded_channel();
      let context = RunContext::new(sender, None);
      let invocation = ToolRequest::new(
        CallId::new("foo"),
        ToolFunction {
          arguments: Value::Null,
          name: ToolName::new("bar").unwrap(),
        },
      );

      assert_eq!(
        policy
          .evaluate(&context, &invocation, ApprovalPolicy::None)
          .await
          .map_err(|error| error.to_string()),
        expected.map_err(str::to_owned)
      );
      assert_eq!(count.load(Ordering::Relaxed), calls);
    }

    case(Vec::new(), Ok(PolicyDecision::Allow), 0).await;
    case(
      vec![
        Ok(PolicyDecision::Allow),
        Ok(PolicyDecision::Ask),
        Ok(PolicyDecision::Allow),
      ],
      Ok(PolicyDecision::Ask),
      3,
    )
    .await;
    case(
      vec![
        Ok(PolicyDecision::Ask),
        Ok(PolicyDecision::Deny {
          reason: "foo".into(),
        }),
        Ok(PolicyDecision::Allow),
      ],
      Ok(PolicyDecision::Deny {
        reason: "foo".into(),
      }),
      2,
    )
    .await;
    case(
      vec![
        Ok(PolicyDecision::Allow),
        Err("foo"),
        Ok(PolicyDecision::Ask),
      ],
      Err("foo"),
      2,
    )
    .await;
  }
}
