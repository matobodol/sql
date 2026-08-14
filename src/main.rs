use std::collections::HashMap;

use coredb::{
    BinaryOp, ColumnConstraint, ColumnPosition, DBM, DataType, DomainError, Expr, Increment,
    SelectStmt, ValueType, catalog::QueryResult,
};

fn main() -> Result<(), DomainError> {
    // 1. Inisialisasi DatabaseManager (secara otomatis membuat folder data/root)
    let mut dbm = DBM::new();

    // // 3. Buat database baru bernama "mydb" di bawah user aktif ('root')
    // dbm.api_database_create("mydb")?;
    dbm.api_database_use("mydb")?;

    #[allow(warnings)]
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
            vec![ColumnConstraint::NotNull],
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
    // dbm.api_table_create("users", raw_columns).unwrap();

    #[allow(warnings)]
    let raw_columns: Vec<(String, DataType, Vec<ColumnConstraint>, ColumnPosition)> = vec![
        (
            "gender".to_string(),
            DataType::Text,
            vec![],
            ColumnPosition::Default,
        ),
        (
            "alamat".to_string(),
            DataType::Text,
            vec![],
            ColumnPosition::First,
        ),
    ];
    dbm.api_column_add("users", raw_columns).unwrap();

    // 5. Panggil save_to_disk untuk menulis metadata.bin dan flush file .db
    #[allow(warnings)]
    let raw_rows = vec![
        vec![
            ValueType::Text("alamat jl abimanyu".into()),
            ValueType::Null,
            ValueType::Text("jono".into()),
            ValueType::Null,
            ValueType::Text("laki laki".into()),
        ],
        vec![
            ValueType::Text("alamat jl abimanyu".into()),
            ValueType::Null,
            ValueType::Text("joni".into()),
            ValueType::Null,
            ValueType::Text("laki laki".into()),
        ],
        vec![
            ValueType::Text("alamat jl abimanyu".into()),
            ValueType::Null,
            ValueType::Text("jani".into()),
            ValueType::Enum {
                type_name: "status".into(),
                value: "lulus".into(),
            },
            ValueType::Text("perempuan".into()),
        ],
    ];
    // dbm.api_row_insert("users", raw_rows).unwrap();

    #[allow(warnings)]
    let raw_rows = vec![
        vec![
            ValueType::Text("alamat jl abimanyu".into()),
            ValueType::Null,
            ValueType::Text("jana".into()),
            ValueType::Null,
            ValueType::Text("laki kai".into()),
        ],
        vec![
            ValueType::Text("alamat jl abimanyu".into()),
            ValueType::Null,
            ValueType::Text("jeni".into()),
            ValueType::Null,
            ValueType::Text("perempuan".into()),
        ],
        vec![
            ValueType::Text("alamat jl abimanyu".into()),
            ValueType::Null,
            ValueType::Text("jejen".into()),
            ValueType::Enum {
                type_name: "status".into(),
                value: "lulus".into(),
            },
            ValueType::Text("perempuan".into()),
        ],
    ];
    // dbm.api_row_insert("users", raw_rows).unwrap();

    #[allow(warnings)]
    let raw_rows = vec![vec![
        ValueType::Text("alamat jl abimanyu".into()),
        ValueType::Null,
        ValueType::Text("alice".into()),
        ValueType::Null,
        ValueType::Text("laki laki".into()),
    ]];
    // dbm.api_row_insert("users", raw_rows).unwrap();

    let mut assignments = HashMap::new();
    assignments.insert(
        "gender".to_string(),
        Expr::Literal(ValueType::Text("perempuan".into())), // Nilai teks baru yang ingin dimasukkan
    );

    // Gunakan predicate untuk menargetkan baris di mana nama sama dengan "alice"
    #[allow(warnings)]
    let predicate = Some(Expr::binary(
        Expr::Column("name".into()),
        BinaryOp::Eq,
        Expr::Literal(ValueType::Text("alice".into())),
    ));

    // dbm.api_row_update("users", assignments, predicate).unwrap();

    // 6. Verifikasi keberadaan seluruh file pada struktur layout persisten
    let statements = SelectStmt {
        projection: Vec::new(),
        selection: None,
        group_by: Vec::new(),
        aggregates: Vec::new(),
        order_by: Vec::new(),
        limit: None,
        offset: 0,
    };

    if let QueryResult::Dql { schema, rows: _ } = dbm.api_select("users", statements).unwrap() {
        println!("{:#?}", schema);
    }

    Ok(())
}
