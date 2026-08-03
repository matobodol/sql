use sql::{ColumnConstraint, ColumnDef, ColumnId, Schema, SqlType, SqlValue, Table, TableId};

fn create_dummy_table() -> Table {
    let col1 = ColumnDef::with_constraints(
        ColumnId(1),
        "id",
        SqlType::Int,
        vec![ColumnConstraint::PrimaryKey],
    );
    let col2 = ColumnDef::new(ColumnId(2), "name", SqlType::Text);
    let schema = Schema::new(vec![col1, col2]).unwrap();

    Table::new(TableId(1), "test_table", schema)
}

#[test]
fn test_table_insert_convenience_methods() {
    let mut table = create_dummy_table();

    // Single Insert Helper
    let res = table.insert(vec![SqlValue::Int(100), SqlValue::Text("John".into())]);
    assert_eq!(res, Ok(1));

    // Batch Insert Helper
    let batch_res = table.insert_batch(vec![
        vec![SqlValue::Int(101), SqlValue::Text("Jane".into())],
        vec![SqlValue::Int(102), SqlValue::Text("Doe".into())],
    ]);
    assert_eq!(batch_res, Ok(2));

    assert_eq!(table.rows().len(), 3);
}

#[test]
fn test_table_rebuild_indexes() {
    let mut table = create_dummy_table();

    table
        .insert(vec![SqlValue::Int(1), SqlValue::Text("A".into())])
        .unwrap();
    table
        .insert(vec![SqlValue::Int(2), SqlValue::Text("B".into())])
        .unwrap();

    // Memastikan rebuild_indexes berjalan tanpa panic/error
    assert!(table.rebuild_indexes().is_ok());

    // Cek BTreeIndex lookup untuk PK (ColumnId(1))
    let index = table.index_registry().get_index(ColumnId(1)).unwrap();
    let row_ids = index.lookup(&SqlValue::Int(2));
    assert_eq!(row_ids.len(), 1);
}
