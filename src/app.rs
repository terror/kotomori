use super::*;

#[derive(Debug)]
pub(crate) struct App {
  agent: Agent,
  event_receiver: UnboundedReceiver<Event>,
  event_sender: UnboundedSender<Event>,
  state: State,
}

impl App {
  const TICK_INTERVAL: Duration = Duration::from_millis(120);

  fn drain_pending_events(&mut self) {
    while let Ok(event) = self.event_receiver.try_recv() {
      self.handle_event(event);
    }
  }

  fn handle_effect(&mut self, effect: Effect) {
    match effect {
      Effect::InterruptAgent => {
        self.agent.interrupt();
      }
      Effect::RunAgent { messages } => {
        self.agent.spawn(messages);
      }
    }
  }

  fn handle_event(&mut self, event: Event) {
    for effect in self.state.handle_event(event) {
      self.handle_effect(effect);
    }
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
    Self::with_state(settings, State::new(settings)?)
  }

  pub(crate) async fn run(mut self) -> Result {
    let mut terminal = Terminal::new()?;

    let mut renderer = Renderer::new();

    self.listen_for_input();

    let mut tick_interval = interval(Self::TICK_INTERVAL);

    while !self.state.should_quit {
      renderer.draw(&mut terminal.stdout, &ViewComponent::new(&self.state))?;

      tokio::select! {
        event = self.event_receiver.recv() => {
          let Some(event) = event else {
            break;
          };

          self.handle_event(event);
        }
        _ = tick_interval.tick() => {
          self.handle_event(Event::Tick(Self::TICK_INTERVAL));
        }
      }

      self.drain_pending_events();
    }

    renderer.finish(&mut terminal.stdout)?;

    Ok(())
  }

  pub(crate) fn with_state(settings: &Settings, state: State) -> Result<Self> {
    let (event_sender, event_receiver) = mpsc::unbounded_channel();

    Ok(Self {
      agent: Agent::new(event_sender.clone(), settings)?,
      event_receiver,
      event_sender,
      state,
    })
  }
}
