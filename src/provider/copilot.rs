use {super::*, ::rig::providers::copilot};

pub(super) fn build(model: &Model) -> Result<Arc<dyn Provider>> {
  let client = copilot::Client::from_env()?;

  Ok(Rig::build(&client, model))
}
