#[cfg(test)]
mod tests {

    use coredb::{
        BinaryOp, ColumnConstraint, DBM, DataType, DomainError, Expr, Increment, SelectStmt,
        ValueType, catalog::QueryResult,
    };

    #[test]
    fn test_persistent_file_layout() -> Result<(), DomainError> {
        // 1. Inisialisasi DatabaseManager (secara otomatis membuat folder data/root)
        let mut dbm = DBM::new();

        // // 3. Buat database baru bernama "test_db" di bawah user aktif ('root')
        dbm.api_database_create("mydb")?;
        dbm.api_database_use("mydb")?;

        let raw_columns = vec![
            (
                "id".to_string(),
                DataType::Int,
                vec![
                    ColumnConstraint::Auto(Increment::Enabled { start: 1, step: 1 }),
                    ColumnConstraint::PrimaryKey,
                ],
            ),
            (
                "name".to_string(),
                DataType::Text,
                vec![ColumnConstraint::NotNull, ColumnConstraint::Unique],
            ),
            (
                "status".to_string(),
                DataType::Enum {
                    name: "status".into(),
                    variants: vec!["lulus".into(), "gagal".to_string()],
                },
                vec![ColumnConstraint::Default(ValueType::Enum {
                    type_name: "status".into(),
                    value: "gagal".into(),
                })],
            ),
        ];
        dbm.api_table_create("users", raw_columns).unwrap();

        // 5. Panggil save_to_disk untuk menulis metadata.bin dan flush file .db
        let raw_rows = vec![
            vec![
                ValueType::Null,
                ValueType::Text("jono".into()),
                ValueType::Null,
            ],
            vec![
                ValueType::Null,
                ValueType::Text("joni".into()),
                ValueType::Null,
            ],
            vec![
                ValueType::Null,
                ValueType::Text("jani".into()),
                ValueType::Enum {
                    type_name: "status".into(),
                    value: "lulus".into(),
                },
            ],
        ];
        dbm.api_row_insert("users", raw_rows).unwrap();

        // 6. Verifikasi keberadaan seluruh file pada struktur layout persisten
        let statements = SelectStmt {
            projection: Vec::new(), // Kosong berarti SELECT * (semua kolom)
            selection: Some(Expr::binary(
                Expr::Column("status".into()),
                BinaryOp::Eq,
                Expr::Literal(ValueType::Enum {
                    type_name: "status".into(),
                    value: "lulus".into(),
                }),
            )),
            group_by: Vec::new(),
            aggregates: Vec::new(),
            order_by: Vec::new(),
            limit: None,
            offset: 0,
        };

        let result = dbm.api_select("users", statements).unwrap();
        match result {
            QueryResult::Dql { schema: s, rows: r } => {
                println!("{:#?}", s);
                println!("{:#?}", r);
            }
            _ => (),
        }

        Ok(())
    }
}
