use super::*;

#[derive(Debug)]
pub(crate) struct App {
  agent: Option<Agent>,
  dimensions: Dimensions,
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
      Effect::RunAgent { messages, run_id } => {
        agent.spawn(run_id, messages);
      }
    }
  }

  fn handle_event(&mut self, event: Event) -> Result {
    if let Event::Resize(dimensions) = event {
      self.dimensions = dimensions;
      return Ok(());
    }

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
        Event::Agent { .. } | Event::Resize(_) | Event::Tick(_) => {}
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

        let Some(event) = Event::from_terminal(&event) else {
          continue;
        };

        if sender.send(event).is_err() {
          return;
        }
      }
    });
  }

  pub(crate) fn new(settings: &Settings) -> Result<Self> {
    Self::with_screen(
      settings,
      Screen::Session(Box::new(State::new(
        Database::new()?,
        Session::new(settings)?,
      )?)),
    )
  }

  pub(crate) fn resume(&mut self, id: i64) -> Result {
    let database = Database::new()?;

    let session = database.load_session(id, &self.settings)?;

    self.agent =
      Some(Agent::new(self.event_sender.clone(), &session.settings)?);

    self.screen = Screen::Session(Box::new(State::new(database, session)?));

    Ok(())
  }

  pub(crate) async fn run(mut self) -> Result {
    let mut renderer = Renderer::new()?;

    let (mut first_draw_started_at, mut first_draw_duration) =
      (FIRST_DRAW_STARTED_AT.get().copied(), None);

    self.listen_for_input();

    let (width, height) =
      crossterm_terminal::size().context("failed to read terminal size")?;

    self.dimensions = Dimensions {
      height: usize::from(height),
      width,
    };

    let mut tick_interval = interval(Self::TICK_INTERVAL);

    while !self.screen.should_quit() {
      renderer.draw(
        &ViewComponent::new(&self.screen, first_draw_duration),
        self.dimensions,
      )?;

      if let Some(started_at) = first_draw_started_at.take() {
        first_draw_duration = Some(started_at.elapsed());
        continue;
      }

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
      dimensions: Dimensions {
        height: 0,
        width: 0,
      },
      event_receiver,
      event_sender,
      screen,
      settings: settings.clone(),
    })
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn resize_events_update_dimensions() {
    let mut app = App::with_screen(
      &Settings {
        model: "mock:foo".parse().unwrap(),
        prompt: None,
        yolo: false,
      },
      Screen::Resume(ResumePicker::new(Vec::new())),
    )
    .unwrap();

    let dimensions = Dimensions {
      height: 24,
      width: 80,
    };

    app.event_sender.send(Event::Resize(dimensions)).unwrap();
    app.drain_pending_events().unwrap();

    assert_eq!(app.dimensions, dimensions);
  }
}
