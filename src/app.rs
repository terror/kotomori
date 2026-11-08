use super::*;

static SYSTEM_PROMPT: LazyLock<String> =
  LazyLock::new(|| include_str!("../etc/system.md").trim_end().to_string());

#[derive(Debug)]
pub(crate) struct App {
  agent: Option<Agent>,
  clipboard: Clipboard,
  database: Database,
  event_receiver: UnboundedReceiver<Event>,
  event_sender: UnboundedSender<Event>,
  screen: Screen,
  settings: Settings,
}

impl App {
  const TICK_INTERVAL: Duration = Duration::from_millis(120);

  fn agent(screen: &Screen) -> Result<Option<Agent>> {
    let Screen::Session(state) = screen else {
      return Ok(None);
    };

    let settings = &state.session.settings;

    let mut tools = ToolRegistry::default();

    tools.register(Arc::new(CommandTool {
      command_executor: CommandExecutor::default(),
      directory: settings.directory.clone(),
    }))?;

    Ok(Some(Agent {
      config: AgentConfig {
        model: settings.model.clone(),
        tools,
        yolo: settings.yolo,
      },
      provider: provider::build(&settings.model)?,
      system: Self::system_prompt(&settings.directory)?,
      task: None,
    }))
  }

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
      Effect::CopyToClipboard(text) => {
        let result =
          self.clipboard.copy(text).map_err(|error| error.to_string());

        self.handle_event(Event::ClipboardCopied(result))?;
      }
      Effect::ForkSession => {
        let Screen::Session(state) = &self.screen else {
          return Ok(());
        };

        let result = now()
          .and_then(|now| self.database.fork_session(&state.session, now))
          .map_err(|error| error.to_string());

        self.handle_event(Event::SessionForked(result))?;
      }
      Effect::InterruptAgent => {
        if let Some(agent) = &mut self.agent {
          agent.interrupt();
        }
      }
      Effect::RunAgent {
        conversation,
        run_id,
      } => {
        if let Some(agent) = &mut self.agent {
          let mut events = agent.spawn(conversation);

          let event_sender = self.event_sender.clone();

          tokio::spawn(async move {
            while let Some(event) = events.recv().await {
              if event_sender.send(Event::Agent { event, run_id }).is_err() {
                break;
              }
            }
          });
        }
      }
      Effect::SaveSession => {
        let Screen::Session(state) = &self.screen else {
          return Ok(());
        };

        let session = &state.session;

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
        Event::Agent { .. }
        | Event::ClipboardCopied(_)
        | Event::SessionForked(_)
        | Event::SessionSaved(_)
        | Event::Tick(_) => {}
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

        let action = match event {
          CrosstermEvent::Key(key) if key.kind == KeyEventKind::Press => {
            Action::from_key(&key)
          }
          CrosstermEvent::Paste(input) => Action::Paste(input),
          _ => continue,
        };

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
      agent: Self::agent(&screen)?,
      clipboard: Clipboard::default(),
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
          screen: &mut self.screen,
        }
        .layout(dimensions)
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
    self.agent = Self::agent(&screen)?;

    self.screen = screen;

    Ok(())
  }

  fn system_prompt(directory: &Path) -> Result<String> {
    let agents = Loader::new(directory).load()?;

    let context = formatdoc! {
      "
      Current working directory: {}

      The command tool runs commands using the platform's system shell. Omit
      `cwd` to use the current working directory. Do not invent absolute paths.
      ",
      directory.display(),
    };

    Ok(
      [
        SYSTEM_PROMPT.as_str(),
        context.trim_end(),
        agents.trim_end(),
      ]
      .into_iter()
      .filter(|section| !section.is_empty())
      .collect::<Vec<_>>()
      .join("\n\n"),
    )
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn system_prompt() {
    fn context(directory: &Path) -> String {
      formatdoc! {
        "
        Current working directory: {}

        The command tool runs commands using the platform's system shell. Omit
        `cwd` to use the current working directory. Do not invent absolute paths.
        ",
        directory.display(),
      }
      .trim_end()
      .to_string()
    }

    #[track_caller]
    fn case<F>(agents: Option<&str>, expected: F)
    where
      F: FnOnce(&Path, &Path) -> String,
    {
      let directory = tempfile::tempdir().unwrap();

      let agents_path = directory.path().join("AGENTS.md");

      if let Some(agents) = agents {
        fs::write(&agents_path, agents).unwrap();
      }

      assert_eq!(
        App::system_prompt(directory.path()).unwrap(),
        expected(directory.path(), &agents_path)
      );
    }

    case(None, |directory, _| {
      format!("{}\n\n{}", SYSTEM_PROMPT.as_str(), context(directory))
    });

    case(Some("foo\n"), |directory, agents_path| {
      format!(
        "{}\n\n{}\n\n{}:\nfoo",
        SYSTEM_PROMPT.as_str(),
        context(directory),
        agents_path.display()
      )
    });
  }
}
