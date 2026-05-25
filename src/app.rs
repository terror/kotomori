use super::*;

#[derive(Debug)]
pub(crate) struct App {
  agent: Agent,
  event_receiver: UnboundedReceiver<Event>,
  event_sender: UnboundedSender<Event>,
  footer: String,
  state: State,
}

impl App {
  fn footer(model: &Model) -> Result<String> {
    let directory =
      env::current_dir().context("failed to read current directory")?;

    let directory = if let Some(home) = env::var_os("HOME").map(PathBuf::from)
      && let Ok(directory) = directory.strip_prefix(home)
    {
      if directory.as_os_str().is_empty() {
        "~".into()
      } else {
        format!("~/{}", directory.display())
      }
    } else {
      directory.display().to_string()
    };

    Ok(format!(
      "{} · {} · {directory}",
      model.provider(),
      model.name()
    ))
  }

  fn handle_effect(&self, effect: Effect) {
    match effect {
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

  pub(crate) fn new(options: Options) -> Result<Self> {
    let (event_sender, event_receiver) = mpsc::unbounded_channel();
    let footer = Self::footer(&options.model)?;

    Ok(Self {
      agent: Agent::new(event_sender.clone(), options.model),
      event_receiver,
      event_sender,
      footer,
      state: State::new(&options.prompt.unwrap_or_default()),
    })
  }

  pub(crate) async fn run(mut self) -> Result {
    let mut terminal = Terminal::new()?;

    let mut renderer = Renderer::new();

    self.listen();
    self.tick();

    while !self.state.should_quit() {
      renderer
        .draw(terminal.stdout_mut(), &View::new(&self.state, &self.footer))?;

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

  fn tick(&self) {
    let sender = self.event_sender.clone();

    tokio::spawn(async move {
      let mut interval = interval(Duration::from_millis(120));

      loop {
        interval.tick().await;

        if sender.send(Event::Tick).is_err() {
          return;
        }
      }
    });
  }
}
