use coredb::{
    ColumnConstraint, CommandAction, DataType, Increment, SystemCatalog, ValueType,
    command::{DmlAction, TableAction},
};

fn main() {
    let user = "user";
    let passwd = "123";
    let mydb = "mydb";
    let mytbl = "mytbl";

    let mut system = SystemCatalog::new();
    system.change_password(None, passwd).unwrap();

    system.create_user(user, passwd).unwrap();
    system.login(user, passwd).unwrap();

    system.create_database(mydb).unwrap();
    system.login("root", passwd).unwrap();

    system.use_database(mydb).unwrap();

    let create_table = CommandAction::TableAction {
        actions: vec![TableAction::CreateTable {
            table_name: mytbl.to_string(),
            columns: vec![
                (
                    "id".to_string(),
                    DataType::Int,
                    vec![ColumnConstraint::Auto(Increment::Enabled {
                        start: 1,
                        step: 1,
                    })],
                ),
                ("name".to_string(), DataType::Text, vec![]),
            ],
        }],
    };
    system.execute(create_table).unwrap();

    let insert = CommandAction::DmlAction {
        table_name: mytbl.into(),
        action: DmlAction::Insert {
            rows: vec![
                vec![ValueType::Null, ValueType::Text("joni".into())],
                vec![ValueType::Null, ValueType::Text("jani".into())],
                vec![ValueType::Null, ValueType::Text("jono".into())],
            ],
        },
    };
    system.execute(insert).unwrap();

    println!("{:#?}", system);
}
