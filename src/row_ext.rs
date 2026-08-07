pub(crate) trait RowExt {
  fn get_u64(self, index: usize) -> rusqlite::Result<u64>;
}

impl RowExt for &rusqlite::Row<'_> {
  fn get_u64(self, index: usize) -> rusqlite::Result<u64> {
    u64::try_from(self.get::<_, i64>(index)?).map_err(|error| {
      rusqlite::Error::FromSqlConversionFailure(
        index,
        rusqlite::types::Type::Integer,
        Box::new(error),
      )
    })
  }
}
