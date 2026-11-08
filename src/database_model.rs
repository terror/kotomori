use super::*;

pub(crate) struct DatabaseModel(pub(crate) Model);

impl FromSql for DatabaseModel {
  fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
    value
      .as_str()?
      .parse::<Model>()
      .map(Self)
      .map_err(|error| FromSqlError::Other(error.into()))
  }
}
