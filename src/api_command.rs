use std::collections::HashMap;

use crate::{
    ColumnConstraint, ColumnPosition, DBM, DataType, DomainError, Expr, Statement, ValueType,
    catalog::QueryResult,
};

pub enum CMD {
    // --- USER ---
    UserLogin {
        username: String,
        password: String,
    },
    CreateUser {
        username: String,
        passwd: String,
    },
    RenameUser {
        old_name: String,
        new_name: String,
    },
    DropUser {
        username: String,
    },
    ChangePassword {
        old_pass: Option<String>,
        new_pass: String,
    },
    ShowUsers,

    // --- DATABASE ---
    CreateDatabasea {
        db_name: String,
    },
    RenamDatabase {
        old_name: String,
        new_name: String,
    },
    DropDatabase {
        db_name: String,
    },
    UseDatabase {
        db_name: String,
    },
    ShowDatabases,

    // --- TABLE ---
    /// (column_name, type data, constraint)
    CreateTable {
        table_name: String,
        raw_columns: Vec<(String, DataType, Vec<ColumnConstraint>)>,
    },
    RenameTable {
        old_name: String,
        new_name: String,
    },
    DropTable {
        table_name: String,
    },
    DescribeTable {
        table_name: String,
    },
    ShowTables,

    // --- COLUMN ---
    AddColumns {
        table_name: String,
        raw_columns: Vec<(String, DataType, Vec<ColumnConstraint>, ColumnPosition)>,
    },
    RenameColumn {
        table_name: String,
        old_name: String,
        new_name: String,
    },
    DropColumn {
        table_name: String,
        column_name: String,
    },
    ModifyType {
        table_name: String,
        column_name: String,
        new_type: DataType,
    },
    AddConstraint {
        table_name: String,
        column_name: String,
        constraint: ColumnConstraint,
    },
    DropConstraint {
        table_name: String,
        column_name: String,
        constraint: ColumnConstraint,
    },
    SetDefault {
        table_name: String,
        column_name: String,
        default_value: Option<ValueType>,
    },

    // --- ROW ---
    Insert {
        table_name: String,
        raw_rows: Vec<Vec<ValueType>>,
    },
    Update {
        table_name: String,
        assignments: HashMap<String, Expr>,
        predicate: Option<Expr>,
    },
    Delete {
        table_name: String,
        predicate: Option<Expr>,
    },

    // --- SELECT ---
    Select {
        table_name: String,
        statement: Statement,
    },
}

pub fn execute(dbm: &mut DBM, commands: Vec<CMD>) -> Result<QueryResult, DomainError> {
    for cmd in commands {
        let result = match cmd {
            // user
            CMD::ShowUsers => dbm.api_user_show(),
            CMD::UserLogin { username, password } => dbm.api_user_login(&username, &password),
            CMD::CreateUser { username, passwd } => dbm.api_user_create(&username, &passwd),
            CMD::DropUser { username } => dbm.api_user_drop(&username),
            CMD::RenameUser { old_name, new_name } => dbm.api_user_rename(&old_name, &new_name),
            CMD::ChangePassword { old_pass, new_pass } => {
                dbm.api_user_change_password(old_pass, &new_pass)
            }

            // database
            CMD::ShowDatabases => dbm.api_databases_show(),
            CMD::UseDatabase { db_name } => dbm.api_database_use(&db_name),
            CMD::CreateDatabasea { db_name } => dbm.api_database_create(&db_name),
            CMD::DropDatabase { db_name } => dbm.api_database_drop(&db_name),
            CMD::RenamDatabase { old_name, new_name } => {
                dbm.api_database_rename(&old_name, &new_name)
            }

            // table
            CMD::CreateTable {
                table_name,
                raw_columns,
            } => dbm.api_table_create(&table_name, raw_columns),
            CMD::RenameTable { old_name, new_name } => dbm.api_table_rename(&old_name, &new_name),
            CMD::DropTable { table_name } => dbm.api_table_drop(&table_name),
            CMD::DescribeTable { table_name } => dbm.api_table_describe(&table_name),
            CMD::ShowTables => dbm.api_table_show(),

            // column
            CMD::AddColumns {
                table_name,
                raw_columns,
            } => dbm.api_column_add(&table_name, raw_columns),
            CMD::DropColumn {
                table_name,
                column_name,
            } => dbm.api_column_drop(&table_name, &column_name),
            CMD::RenameColumn {
                table_name,
                old_name,
                new_name,
            } => dbm.api_column_rename(&table_name, &old_name, &new_name),
            CMD::ModifyType {
                table_name,
                column_name,
                new_type,
            } => dbm.api_column_modify_type(&table_name, &column_name, new_type),
            CMD::AddConstraint {
                table_name,
                column_name,
                constraint,
            } => dbm.api_column_constraint_add(&table_name, &column_name, constraint),
            CMD::DropConstraint {
                table_name,
                column_name,
                constraint,
            } => dbm.api_column_constraint_drop(&table_name, &column_name, constraint),
            CMD::SetDefault {
                table_name,
                column_name,
                default_value,
            } => dbm.api_column_set_default(&table_name, &column_name, default_value),

            // row
            CMD::Insert {
                table_name,
                raw_rows,
            } => dbm.api_row_insert(&table_name, raw_rows),
            CMD::Update {
                table_name,
                assignments,
                predicate,
            } => dbm.api_row_update(&table_name, assignments, predicate),
            CMD::Delete {
                table_name,
                predicate,
            } => dbm.api_row_delete(&table_name, predicate),

            // select
            CMD::Select {
                table_name,
                statement,
            } => dbm.api_select(&table_name, statement),
        };

        if result.is_err() {
            return result;
        }
    }

    Ok(QueryResult::OK)
}
