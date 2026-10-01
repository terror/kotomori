use super::*;

#[derive(Debug)]
pub(crate) struct App {
  agent: Option<Agent>,
  dimensions: Dimensions,
  event_receiver: UnboundedReceiver<Event>,
  event_sender: UnboundedSender<Event>,
  redraw: bool,
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

  fn draw(
    &mut self,
    renderer: &mut Renderer<impl Write>,
    first_draw_duration: Option<Duration>,
  ) -> Result<bool> {
    if !mem::take(&mut self.redraw) {
      return Ok(false);
    }

    renderer.draw(
      &ViewComponent::new(&self.screen, first_draw_duration),
      self.dimensions,
    )?;

    Ok(true)
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
      self.redraw |= self.dimensions != dimensions;
      self.dimensions = dimensions;
      return Ok(());
    }

    match &mut self.screen {
      Screen::Quit => {}
      Screen::Resume(picker) => match event {
        Event::Action(action) => {
          let action = picker.handle_action(action);
          self.redraw |= picker.take_redraw();

          let Some(action) = action else {
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
        self.redraw |= state.take_redraw();

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

  fn needs_animation(&self) -> bool {
    match &self.screen {
      Screen::Session(state) => {
        state.active_run().is_some_and(Run::needs_animation)
      }
      Screen::Quit | Screen::Resume(_) => false,
    }
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
    self.redraw = true;

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
    tick_interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

    let mut animating = false;

    while !self.screen.should_quit() {
      let needs_animation = self.needs_animation();

      if needs_animation && !animating {
        tick_interval.reset();
      }

      animating = needs_animation;

      if self.draw(&mut renderer, first_draw_duration)?
        && let Some(started_at) = first_draw_started_at.take()
      {
        first_draw_duration = Some(started_at.elapsed());
        self.redraw = true;
        continue;
      }

      tokio::select! {
        event = self.event_receiver.recv() => {
          let Some(event) = event else {
            break;
          };

          self.handle_event(event)?;
        }
        _ = tick_interval.tick(), if animating => {
          self.handle_event(Event::Tick(Instant::now()))?;
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
      redraw: true,
      screen,
      settings: settings.clone(),
    })
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn draws_only_after_visible_events_and_coalesces_pending_events() {
    let settings = Settings {
      model: "mock:foo".parse().unwrap(),
      prompt: None,
      yolo: false,
    };

    for screen in [
      Screen::Resume(ResumePicker::new(Vec::new())),
      Screen::Session(Box::new(
        State::new(Database::new().unwrap(), Session::new(&settings).unwrap())
          .unwrap(),
      )),
    ] {
      let mut app = App::with_screen(&settings, screen).unwrap();
      let mut renderer = Renderer::<Vec<u8>>::default();
      let dimensions = Dimensions {
        height: 24,
        width: 80,
      };

      app.handle_event(Event::Resize(dimensions)).unwrap();
      assert!(app.draw(&mut renderer, None).unwrap());
      assert!(!app.draw(&mut renderer, None).unwrap());
      assert!(!app.needs_animation());

      for event in [
        Event::Tick(Instant::now()),
        Event::Resize(dimensions),
        Event::Action(Action::Edit(Input {
          key: Key::Backspace,
          ..Default::default()
        })),
        Event::Action(Action::Submit),
        Event::Agent {
          event: AgentEvent::Done,
          run_id: 0,
        },
      ] {
        app.handle_event(event).unwrap();
        assert!(!app.draw(&mut renderer, None).unwrap());
      }

      for c in "foo".chars() {
        app
          .event_sender
          .send(Event::Action(Action::Edit(Input {
            key: Key::Char(c),
            ..Default::default()
          })))
          .unwrap();
      }

      app.drain_pending_events().unwrap();
      assert!(app.draw(&mut renderer, None).unwrap());
      assert!(!app.draw(&mut renderer, None).unwrap());

      app
        .handle_event(Event::Resize(Dimensions {
          width: 40,
          ..dimensions
        }))
        .unwrap();
      assert!(app.draw(&mut renderer, None).unwrap());
      assert!(!app.draw(&mut renderer, None).unwrap());
    }
  }

  #[test]
  fn ticks_draw_only_visible_animation() {
    let settings = Settings {
      model: "mock:foo".parse().unwrap(),
      prompt: Some("foo".into()),
      yolo: false,
    };
    let mut state =
      State::new(Database::new().unwrap(), Session::new(&settings).unwrap())
        .unwrap();
    state.handle_event(Event::Action(Action::Submit));
    state.take_redraw();
    let mut app =
      App::with_screen(&settings, Screen::Session(Box::new(state))).unwrap();
    let mut renderer = Renderer::<Vec<u8>>::default();
    app
      .handle_event(Event::Resize(Dimensions {
        height: 24,
        width: 80,
      }))
      .unwrap();

    for (event, animating) in [
      (Event::Tick(Instant::now()), true),
      (
        Event::Agent {
          event: AgentEvent::Update(MessageUpdate::Text {
            index: 0,
            delta: "bar".into(),
          }),
          run_id: 0,
        },
        false,
      ),
      (
        Event::Agent {
          event: AgentEvent::Update(MessageUpdate::ReasoningDelta {
            index: 1,
            delta: "baz".into(),
          }),
          run_id: 0,
        },
        true,
      ),
      (
        Event::Agent {
          event: AgentEvent::Done,
          run_id: 0,
        },
        false,
      ),
    ] {
      app.handle_event(event).unwrap();
      assert_eq!(app.needs_animation(), animating);
      assert!(app.draw(&mut renderer, None).unwrap());
      app
        .handle_event(Event::Agent {
          event: AgentEvent::Update(MessageUpdate::MessageId("foo".into())),
          run_id: 0,
        })
        .unwrap();
      assert!(!app.draw(&mut renderer, None).unwrap());
      app.handle_event(Event::Tick(Instant::now())).unwrap();
      assert_eq!(app.draw(&mut renderer, None).unwrap(), animating);
    }
  }

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
