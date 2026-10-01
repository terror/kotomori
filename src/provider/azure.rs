use {super::*, ::rig::providers::azure};

pub(super) fn build(model: &Model) -> Result<Arc<dyn Provider>> {
  let client = azure::from_env()?;

  Ok(Rig::build(client.completion(&model.name)))
}
