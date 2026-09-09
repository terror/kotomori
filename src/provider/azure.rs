use {super::*, ::rig::providers::azure};

pub(super) fn build(model: &Model) -> Result<Arc<dyn Provider>> {
  let client = azure::Client::from_env()?;

  Ok(Rig::build(&client, model))
}
