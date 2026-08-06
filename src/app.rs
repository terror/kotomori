use super::*;

#[derive(Debug)]
pub(crate) struct App {
  agent: Option<Agent>,
  event_receiver: UnboundedReceiver<Event>,
  event_sender: UnboundedSender<Event>,
  screen: Screen,
  settings: Settings,
}

impl App {
  const TICK_INTERVAL: Duration = Duration::from_millis(120);

  fn drain_pending_events(&mut self) -> Result {
    while let Ok(event) = self.event_receiver.try_recv() {
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
            ResumePickerAction::Resume(path) => self.resume(&path)?,
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
    let sender = self.event_sender.clone();

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

  fn resume(&mut self, path: &Path) -> Result {
    let session = SessionStore::load(path)?;

    let mut settings = self.settings.clone();

    settings.model = session.file.model.parse().with_context(|| {
      format!("failed to parse session model {}", session.file.model)
    })?;

    self.agent = Some(Agent::new(self.event_sender.clone(), &settings)?);

    self.screen =
      Screen::Session(Box::new(State::with_session(&settings, session)?));

    self.settings = settings;

    Ok(())
  }

  pub(crate) async fn run(mut self) -> Result {
    let mut terminal = Terminal::new()?;

    let mut renderer = Renderer::new();

    self.listen_for_input();

    let mut tick_interval = interval(Self::TICK_INTERVAL);

    while !self.screen.should_quit() {
      renderer.draw(&mut terminal.stdout, &ViewComponent::new(&self.screen))?;

      tokio::select! {
        event = self.event_receiver.recv() => {
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

    renderer.finish(&mut terminal.stdout)?;

    Ok(())
  }

  pub(crate) fn with_screen(
    settings: &Settings,
    screen: Screen,
  ) -> Result<Self> {
    let (event_sender, event_receiver) = mpsc::unbounded_channel();

    let agent = if matches!(screen, Screen::Session(_)) {
      Some(Agent::new(event_sender.clone(), settings)?)
    } else {
      None
    };

    Ok(Self {
      agent,
      event_receiver,
      event_sender,
      screen,
      settings: settings.clone(),
    })
  }
}
