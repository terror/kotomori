use super::*;

#[derive(Debug)]
pub(crate) struct App {
  agent: Agent,
  event_receiver: UnboundedReceiver<Event>,
  event_sender: UnboundedSender<Event>,
  state: State,
}

impl App {
  fn handle_effect(&self, effect: Effect) {
    match effect {
      Effect::RunAgent { input } => {
        self.agent.spawn(input);
      }
    }
  }

  fn handle_event(&mut self, event: Event) {
    for effect in self.state.handle_event(event) {
      self.handle_effect(effect);
    }
  }

  fn listen(&self) {
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

  pub(crate) fn new(options: Options) -> Self {
    let (event_sender, event_receiver) = mpsc::unbounded_channel();
    let prompt = options.prompt.unwrap_or_default();

    Self {
      agent: Agent::new(event_sender.clone(), options.model),
      event_receiver,
      event_sender,
      state: State::new(&prompt),
    }
  }

  pub(crate) async fn run(mut self) -> Result {
    let mut terminal = Terminal::new()?;

    let mut renderer = Renderer::new();

    self.listen();

    while !self.state.should_quit() {
      renderer.draw(terminal.stdout_mut(), &View::new(&self.state))?;

      let Some(event) = self.event_receiver.recv().await else {
        break;
      };

      self.handle_event(event);

      while let Ok(event) = self.event_receiver.try_recv() {
        self.handle_event(event);
      }
    }

    Ok(())
  }
}
