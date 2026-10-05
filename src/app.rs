use super::*;

#[derive(Debug)]
pub(crate) struct App {
  agent: Option<Agent>,
  database: Database,
  event_receiver: UnboundedReceiver<Event>,
  event_sender: UnboundedSender<Event>,
  screen: Screen,
  settings: Settings,
}

impl App {
  const TICK_INTERVAL: Duration = Duration::from_millis(120);

  fn drain_pending_events(&mut self) -> Result {
    while !self.screen.should_quit() {
      let Ok(event) = self.event_receiver.try_recv() else {
        break;
      };

      self.handle_event(event)?;
    }

    Ok(())
  }

  fn handle_effect(&mut self, effect: Effect) -> Result {
    match effect {
      Effect::InterruptAgent => {
        if let Some(agent) = &mut self.agent {
          agent.interrupt();
        }
      }
      Effect::RunAgent { messages, run_id } => {
        if let Some(agent) = &mut self.agent {
          agent.spawn(run_id, messages);
        }
      }
      Effect::SaveSession => {
        let Screen::Session(state) = &self.screen else {
          return Ok(());
        };

        let session = state.session();

        if session.transcript.is_empty() && session.id.is_none() {
          return Ok(());
        }

        let result = now()
          .and_then(|updated_at| {
            self.database.save_session(session, updated_at)
          })
          .map_err(|error| error.to_string());

        self.handle_event(Event::SessionSaved(result))?;
      }
    }

    Ok(())
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
            ResumePickerAction::Cancel => self.set_screen(Screen::Quit)?,
            ResumePickerAction::Resume(id) => self.resume(id)?,
          }
        }
        Event::Error(error) => bail!("failed to read terminal input: {error}"),
        Event::Agent { .. } | Event::SessionSaved(_) | Event::Tick(_) => {}
      },
      Screen::Session(state) => {
        let effects = state.handle_event(event);

        for effect in effects {
          self.handle_effect(effect)?;
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

  pub(crate) fn new(database: Database, settings: &Settings) -> Result<Self> {
    let (event_sender, event_receiver) = mpsc::unbounded_channel();

    let screen =
      Screen::Session(Box::new(State::new(Session::new(settings, now()?))));

    Ok(Self {
      agent: screen.agent(event_sender.clone())?,
      database,
      event_receiver,
      event_sender,
      screen,
      settings: settings.clone(),
    })
  }

  pub(crate) fn resume(&mut self, id: i64) -> Result {
    self.set_screen(Screen::Session(Box::new(State::new(
      self.database.load_session(id, &self.settings)?,
    ))))
  }

  pub(crate) async fn run(mut self) -> Result {
    let mut renderer = Renderer::new()?;

    let (mut first_draw_started_at, mut first_draw_duration) =
      (FIRST_DRAW_STARTED_AT.get().copied(), None);

    self.listen_for_input();

    let mut tick_interval = interval(Self::TICK_INTERVAL);

    while !self.screen.should_quit() {
      renderer.draw(|dimensions| {
        ViewComponent {
          first_draw_duration,
          screen: &self.screen,
        }
        .render(dimensions)
      })?;

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

  pub(crate) fn set_screen(&mut self, screen: Screen) -> Result {
    self.agent = screen.agent(self.event_sender.clone())?;

    self.screen = screen;

    Ok(())
  }
}
