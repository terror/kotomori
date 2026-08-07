use super::*;

#[derive(Debug)]
pub(crate) struct App {
  agent: Option<Agent>,
  event_channel: Channel<Event>,
  screen: Screen,
  settings: Settings,
}

impl App {
  const TICK_INTERVAL: Duration = Duration::from_millis(120);

  fn drain_pending_events(&mut self) -> Result {
    while let Some(event) = self.event_channel.try_recv() {
      self.handle_event(event)?;
    }

    Ok(())
  }

  fn handle_effect(&mut self, effect: Effect) {
    let Some(agent) = &mut self.agent else {
      return;
    };

    match effect {
      Effect::InterruptAgent => {
        agent.interrupt();
      }
      Effect::RunAgent { messages } => {
        agent.spawn(messages);
      }
    }
  }

  fn handle_event(&mut self, event: Event) -> Result {
    match &mut self.screen {
      Screen::Quit => {}
      Screen::Resume(picker) => match event {
        Event::Action(action) => {
          let Some(action) = picker.handle_action(action) else {
            return Ok(());
          };

          match action {
            ResumePickerAction::Cancel => self.screen = Screen::Quit,
            ResumePickerAction::Resume(id) => self.resume(id)?,
          }
        }
        Event::Error(error) => bail!("failed to read terminal input: {error}"),
        Event::AgentDelta(_)
        | Event::AgentDone
        | Event::AgentReasoningDelta(_)
        | Event::AgentToolCall(_)
        | Event::AgentToolResult { .. }
        | Event::Tick(_)
        | Event::ToolApprovalRequest(_) => {}
      },
      Screen::Session(state) => {
        let effects = state.handle_event(event);

        for effect in effects {
          self.handle_effect(effect);
        }
      }
    }

    Ok(())
  }

  fn listen_for_input(&self) {
    let sender = self.event_channel.sender();

    thread::spawn(move || {
      loop {
        let event = match crossterm_event::read() {
          Ok(event) => event,
          Err(error) => {
            let _ = sender.send(Event::Error(error.to_string()));
            return;
          }
        };

        let CrosstermEvent::Key(key) = event else {
          continue;
        };

        if key.kind != KeyEventKind::Press {
          continue;
        }

        let action = Action::from_key(&key);

        if sender.send(Event::Action(action)).is_err() {
          return;
        }
      }
    });
  }

  pub(crate) fn new(settings: &Settings) -> Result<Self> {
    Self::with_screen(
      settings,
      Screen::Session(Box::new(State::new(settings)?)),
    )
  }

  fn resume(&mut self, id: i64) -> Result {
    let database = Database::new()?;

    let session = database.load_session(id)?;

    let mut settings = self.settings.clone();

    settings.model = session.model.parse().with_context(|| {
      format!("failed to parse session model {}", session.model)
    })?;

    self.agent = Some(Agent::new(self.event_channel.sender(), &settings)?);

    self.screen = Screen::Session(Box::new(State::with_session(
      &settings, database, session,
    )?));

    self.settings = settings;

    Ok(())
  }

  pub(crate) async fn run(mut self) -> Result {
    let mut renderer = Renderer::new()?;

    let (mut first_draw_started_at, mut first_draw_duration) =
      (FIRST_DRAW_STARTED_AT.get().copied(), None);

    self.listen_for_input();

    let mut tick_interval = interval(Self::TICK_INTERVAL);

    while !self.screen.should_quit() {
      renderer.draw(&ViewComponent::new(&self.screen, first_draw_duration))?;

      if let Some(started_at) = first_draw_started_at.take() {
        first_draw_duration = Some(started_at.elapsed());
        continue;
      }

      tokio::select! {
        event = self.event_channel.recv() => {
          let Some(event) = event else {
            break;
          };

          self.handle_event(event)?;
        }
        _ = tick_interval.tick() => {
          self.handle_event(Event::Tick(Self::TICK_INTERVAL))?;
        }
      }

      self.drain_pending_events()?;
    }

    Ok(())
  }

  pub(crate) fn with_screen(
    settings: &Settings,
    screen: Screen,
  ) -> Result<Self> {
    let event_channel = Channel::new();

    let agent = if matches!(screen, Screen::Session(_)) {
      Some(Agent::new(event_channel.sender(), settings)?)
    } else {
      None
    };

    Ok(Self {
      agent,
      event_channel,
      screen,
      settings: settings.clone(),
    })
  }
}
