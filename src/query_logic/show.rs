use std::sync::Arc;

use crate::{
    Column, ColumnId, DomainError, QueryResult, Row, RowId, Schema, SqlType, SqlValue,
    catalog::CatalogStore,
};

pub(crate) fn show_tables(catalog: &CatalogStore) -> Result<QueryResult, DomainError> {
    let col_def = Column::new(ColumnId(1), "table_name", SqlType::Text);
    let schema = Schema::new(vec![col_def])?;

    let table_names = catalog.list_tables();
    let mut rows = Vec::with_capacity(table_names.len());

    for (idx, name) in table_names.into_iter().enumerate() {
        let row_id = RowId((idx + 1) as u64);
        let values = vec![SqlValue::Text(Arc::from(name))];
        rows.push(Row::with_id(row_id, values));
    }

    Ok(QueryResult::Dql { schema, rows })
}
