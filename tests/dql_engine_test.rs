#[cfg(test)]
mod tests {
    use sql::catalog::table::Table;
    use sql::domain::id::{ColumnId, TableId};
    use sql::domain::{ColumnDef, Row, Schema, SqlType, SqlValue};
    use sql::execution::sort::{OrderByExpr, SortOrder};
    use sql::expr::Expr;
    use sql::{BinaryOp, SelectStmt, execute_select};

    fn setup_test_table() -> Table {
        // 1. Buat Skema Tabel: id (Int), name (Text), age (Int)
        let schema = Schema::new(vec![
            ColumnDef::new(ColumnId(1), "id", SqlType::Int),
            ColumnDef::new(ColumnId(2), "name", SqlType::Text),
            ColumnDef::new(ColumnId(3), "age", SqlType::Int),
        ])
        .unwrap();

        // 2. Buat instance tabel kosong menggunakan konstruktor Table::new
        let mut table = Table::new(TableId(1), "users", schema);

        // 3. Masukkan baris data uji coba
        let rows = vec![
            Row::new(vec![
                SqlValue::Int(1),
                SqlValue::Text("Alice".into()),
                SqlValue::Int(25),
            ]),
            Row::new(vec![
                SqlValue::Int(2),
                SqlValue::Text("Bob".into()),
                SqlValue::Int(30),
            ]),
            Row::new(vec![
                SqlValue::Int(3),
                SqlValue::Text("Charlie".into()),
                SqlValue::Int(22),
            ]),
            Row::new(vec![
                SqlValue::Int(4),
                SqlValue::Text("Diana".into()),
                SqlValue::Int(28),
            ]),
        ];

        // Masukkan baris ke tabel (sesuaikan dengan method insert/mutasi baris yang ada di struct Table kamu)
        for row in rows {
            table.rows_mut().push(row); // Atau metode insert row yang sesuai
        }

        table
    }

    #[test]
    fn test_select_basic_and_filter() {
        let table = setup_test_table();

        // Query: SELECT * WHERE age > 24
        let stmt = SelectStmt {
            projection: vec![
                Expr::Column(ColumnId(1)),
                Expr::Column(ColumnId(2)),
                Expr::Column(ColumnId(3)),
            ],
            // WHERE age > 24
            selection: Some(Expr::binary(
                Expr::Column(ColumnId(3)),
                BinaryOp::Gt,
                Expr::lit(24),
            )),
            group_by: vec![],
            aggregates: vec![],
            order_by: vec![],
            limit: None,
            offset: 0,
        };

        let result = execute_select(&table, stmt).unwrap();

        // Harusnya Alice (25), Bob (30), Diana (28) -> Total 3 baris
        assert_eq!(result.rows.len(), 3);
        assert_eq!(result.rows[0].values()[1], SqlValue::Text("Alice".into()));
        assert_eq!(result.rows[1].values()[1], SqlValue::Text("Bob".into()));
        assert_eq!(result.rows[2].values()[1], SqlValue::Text("Diana".into()));
    }

    #[test]
    fn test_select_sort_and_limit() {
        let table = setup_test_table();

        // Query: SELECT name, age ORDER BY age ASC LIMIT 2
        let stmt = SelectStmt {
            projection: vec![
                Expr::Column(ColumnId(2)), // name
                Expr::Column(ColumnId(3)), // age
            ],
            selection: None,
            group_by: vec![],
            aggregates: vec![],
            order_by: vec![OrderByExpr {
                expr: Expr::Column(ColumnId(3)), // ORDER BY age
                order: SortOrder::Ascending,
            }],
            limit: Some(2), // LIMIT 2
            offset: 0,
        };

        let result = execute_select(&table, stmt).unwrap();

        // Urutan umur terkecil: Charlie (22), Alice (25). Karena LIMIT 2, Diana dan Bob tidak ikut.
        assert_eq!(result.rows.len(), 2);
        assert_eq!(result.rows[0].values()[0], SqlValue::Text("Charlie".into()));
        assert_eq!(result.rows[1].values()[0], SqlValue::Text("Alice".into()));
    }
}
