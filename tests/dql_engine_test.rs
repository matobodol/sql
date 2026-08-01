use std::sync::Arc;

use ordered_float::OrderedFloat;

// Adjust import sesuai nama crate milikmu
use sql::{
    binary_op::BinaryOp,
    catalog::registry::SymbolRegistry,
    domain::{ColumnDef, DomainError, Row, Schema, SqlType, SqlValue, id::ColumnId},
    execution::{
        AggregateFunc, AggregateOperator, FilterOperator, LimitOperator, OrderByExpr,
        PhysicalOperator, ProjectionOperator, SeqScanOperator, SortOperator, SortOrder,
    },
    expr::Expr,
};

/// Helper untuk membuat Schema dan data Dummy menggunakan SymbolRegistry
fn setup_dummy_environment() -> (Schema, Vec<Row>, ColumnId, ColumnId, ColumnId, ColumnId) {
    let mut registry = SymbolRegistry::new();
    let table_id = registry.register_table("users").unwrap();

    // Register Kolom
    let col_id = registry.register_column(table_id, "id");
    let col_name = registry.register_column(table_id, "name");
    let col_score = registry.register_column(table_id, "score");
    let col_active = registry.register_column(table_id, "is_active");

    let columns = vec![
        ColumnDef::new(col_id, "id", SqlType::Int),
        ColumnDef::new(col_name, "name", SqlType::Text),
        ColumnDef::new(col_score, "score", SqlType::Float),
        ColumnDef::new(col_active, "is_active", SqlType::Bool),
    ];

    let schema = Schema::new(columns).unwrap();

    // Mocking Data Baris (Row)
    let rows = vec![
        Row::new(vec![
            SqlValue::Int(1),
            SqlValue::Text("Alice".into()),
            SqlValue::Float(OrderedFloat(85.5)),
            SqlValue::Bool(true),
        ]),
        Row::new(vec![
            SqlValue::Int(2),
            SqlValue::Text("Bob".into()),
            SqlValue::Null,
            SqlValue::Bool(true),
        ]),
        Row::new(vec![
            SqlValue::Int(3),
            SqlValue::Text("Charlie".into()),
            SqlValue::Float(OrderedFloat(92.0)),
            SqlValue::Bool(false),
        ]),
        Row::new(vec![
            SqlValue::Int(4),
            SqlValue::Text("Diana".into()),
            SqlValue::Float(OrderedFloat(70.0)),
            SqlValue::Bool(true),
        ]),
        Row::new(vec![
            SqlValue::Int(5),
            SqlValue::Text("Eve".into()),
            SqlValue::Null,
            SqlValue::Bool(false),
        ]),
    ];

    (schema, rows, col_id, col_name, col_score, col_active)
}

#[test]
fn test_volcano_scan_and_filter() -> Result<(), DomainError> {
    let (schema, rows, _col_id, _col_name, col_score, col_active) = setup_dummy_environment();

    // Bungkus rows ke dalam Arc
    let rows_arc = Arc::new(rows);

    // Tidak perlu anotasi lifetime `'_`, Box sepenuhnya 'static dan bersih!
    let scan = Box::new(SeqScanOperator::new(rows_arc, schema));

    // Predikat: is_active = true AND score > 80.0
    let pred_active = Expr::Binary {
        left: Box::new(Expr::Column(col_active)),
        op: BinaryOp::Eq,
        right: Box::new(Expr::Literal(SqlValue::Bool(true))),
    };
    let pred_score = Expr::Binary {
        left: Box::new(Expr::Column(col_score)),
        op: BinaryOp::Gt,
        right: Box::new(Expr::Literal(SqlValue::Float(OrderedFloat(80.0)))),
    };
    let combined_pred = Expr::Binary {
        left: Box::new(pred_active),
        op: BinaryOp::And,
        right: Box::new(pred_score),
    };

    let mut filter = FilterOperator::new(scan, combined_pred);

    // Evaluasi Baris 1: Alice (score 85.5, active true) -> Lolos Filter
    let row1 = filter.next()?.expect("Harus menemukan baris Alice");
    assert_eq!(row1[1], SqlValue::Text("Alice".into()));

    // Baris Bob (score NULL -> 3VL Unknown) & Charlie (active false) tersaring
    // Baris Diana (score 70.0 <= 80.0) tersaring

    // End of Stream Iterator
    assert!(filter.next()?.is_none());

    Ok(())
}

#[test]
fn test_volcano_pipeline_select_sort_limit() -> Result<(), DomainError> {
    let (schema, rows, _col_id, col_name, col_score, _col_active) = setup_dummy_environment();

    // Bungkus rows ke dalam Arc
    let rows_arc = Arc::new(rows);

    // Tidak perlu anotasi lifetime `'_`, Box sepenuhnya 'static dan bersih!
    let scan = Box::new(SeqScanOperator::new(rows_arc, schema));

    // Projection Schema (name, score)
    let proj_schema = Schema::new(vec![
        ColumnDef::new(col_name, "name", SqlType::Text),
        ColumnDef::new(col_score, "score", SqlType::Float),
    ])?;
    let proj_exprs = vec![Expr::Column(col_name), Expr::Column(col_score)];
    let proj = Box::new(ProjectionOperator::new(scan, proj_exprs, proj_schema));

    // Sort ORDER BY score DESC
    let sort_spec = vec![OrderByExpr {
        expr: Expr::Column(col_score),
        order: SortOrder::Descending,
    }];
    let sort = Box::new(SortOperator::new(proj, sort_spec));

    // LIMIT 2
    let mut limit = LimitOperator::new(sort, Some(2), 0);

    // Hasil 1: Charlie ( score 92.0 )
    let r1 = limit.next()?.expect("Harus ada baris ke-1");
    assert_eq!(r1[0], SqlValue::Text("Charlie".into()));

    // Hasil 2: Alice ( score 85.5 )
    let r2 = limit.next()?.expect("Harus ada baris ke-2");
    assert_eq!(r2[0], SqlValue::Text("Alice".into()));

    // Baris ke-3 dan seterusnya terpotong oleh LIMIT 2
    assert!(limit.next()?.is_none());

    Ok(())
}

#[test]
fn test_volcano_aggregate_group_by() -> Result<(), DomainError> {
    let (schema, rows, _col_id, _col_name, col_score, col_active) = setup_dummy_environment();

    // Bungkus rows ke dalam Arc
    let rows_arc = Arc::new(rows);

    // Tidak perlu anotasi lifetime `'_`, Box sepenuhnya 'static dan bersih!
    let scan = Box::new(SeqScanOperator::new(rows_arc, schema));

    let dummy_agg_count_id = ColumnId(100);
    let dummy_agg_min_id = ColumnId(101);
    let dummy_agg_max_id = ColumnId(102);

    let out_schema = Schema::new(vec![
        ColumnDef::new(col_active, "is_active", SqlType::Bool),
        ColumnDef::new(dummy_agg_count_id, "count", SqlType::Int),
        ColumnDef::new(dummy_agg_min_id, "min_score", SqlType::Float),
        ColumnDef::new(dummy_agg_max_id, "max_score", SqlType::Float),
    ])?;

    let aggregates = vec![
        AggregateFunc::Count(None),
        AggregateFunc::Min(col_score),
        AggregateFunc::Max(col_score),
    ];

    let mut agg = AggregateOperator::new(
        scan,
        vec![col_active], // Group By is_active
        aggregates,
        out_schema,
    );

    // Ambil hasil pengelompokan (Ada 2 group: true dan false)
    let mut results = Vec::new();
    while let Some(row) = agg.next()? {
        results.push(row);
    }

    assert_eq!(results.len(), 2);

    // Verifikasi Group is_active = true (Alice [85.5], Bob [NULL], Diana [70.0])
    // Expected: COUNT(*) = 3, MIN = 70.0, MAX = 85.5
    let active_group = results
        .iter()
        .find(|r| r[0] == SqlValue::Bool(true))
        .unwrap();
    assert_eq!(active_group[1], SqlValue::Int(3)); // COUNT(*)
    assert_eq!(active_group[2], SqlValue::Float(OrderedFloat(70.0))); // MIN
    assert_eq!(active_group[3], SqlValue::Float(OrderedFloat(85.5))); // MAX

    Ok(())
}

#[test]
fn test_volcano_3vl_null_filtering() -> Result<(), DomainError> {
    let (schema, rows, _col_id, _col_name, col_score, _col_active) = setup_dummy_environment();

    // Bungkus rows ke dalam Arc
    let rows_arc = Arc::new(rows);

    // Tidak perlu anotasi lifetime `'_`, Box sepenuhnya 'static dan bersih!
    let scan = Box::new(SeqScanOperator::new(rows_arc, schema));

    let pred_eq = Expr::Binary {
        left: Box::new(Expr::Column(col_score)),
        op: BinaryOp::Eq,
        right: Box::new(Expr::Literal(SqlValue::Float(OrderedFloat(85.5)))),
    };
    let pred_null = Expr::IsNull(Box::new(Expr::Column(col_score)));

    let combined = Expr::Binary {
        left: Box::new(pred_eq),
        op: BinaryOp::Or,
        right: Box::new(pred_null),
    };

    let mut filter = FilterOperator::new(scan, combined);

    let mut matched_names = Vec::new();
    while let Some(row) = filter.next()? {
        matched_names.push(row[1].clone());
    }

    // Alice (85.5), Bob (NULL), Eve (NULL) -> 3 Baris Lolos Predikat
    assert_eq!(matched_names.len(), 3);
    assert!(matched_names.contains(&SqlValue::Text("Alice".into())));
    assert!(matched_names.contains(&SqlValue::Text("Bob".into())));
    assert!(matched_names.contains(&SqlValue::Text("Eve".into())));

    Ok(())
}
