use {super::*, ::rig::providers::chatgpt};

pub(super) fn build(model: &Model) -> Result<Arc<dyn Provider>> {
  let client = chatgpt::Client::from_env()?;

  Ok(Rig::build(&client, model))
}
