use std::collections::HashMap;

use crate::{
    ColumnConstraint, ColumnPosition, DBM, DataType, DomainError, Expr, ValueType,
    catalog::QueryResult, logic::Statement,
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
    CreateDatabase {
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

pub(crate) fn execute(dbm: &mut DBM, commands: Vec<CMD>) -> Result<QueryResult, DomainError> {
    for cmd in commands {
        let result = match cmd {
            // user
            CMD::ShowUsers => dbm.show_users(),
            CMD::UserLogin { username, password } => dbm.user_login(&username, &password),
            CMD::CreateUser { username, passwd } => dbm.create_user(&username, &passwd),
            CMD::DropUser { username } => dbm.drop_user(&username),
            CMD::RenameUser { old_name, new_name } => dbm.rename_user(&old_name, &new_name),
            CMD::ChangePassword { old_pass, new_pass } => dbm.change_password(old_pass, &new_pass),

            // database
            CMD::ShowDatabases => dbm.show_databases(),
            CMD::UseDatabase { db_name } => dbm.use_database(&db_name),
            CMD::CreateDatabase { db_name } => dbm.create_database(&db_name),
            CMD::DropDatabase { db_name } => dbm.drop_database(&db_name),
            CMD::RenamDatabase { old_name, new_name } => dbm.rename_database(&old_name, &new_name),

            // table
            CMD::CreateTable {
                table_name,
                raw_columns,
            } => dbm.create_table(&table_name, raw_columns),
            CMD::RenameTable { old_name, new_name } => dbm.rename_table(&old_name, &new_name),
            CMD::DropTable { table_name } => dbm.drop_table(&table_name),
            CMD::DescribeTable { table_name } => dbm.describe_table(&table_name),
            CMD::ShowTables => dbm.show_tables(),

            // column
            CMD::AddColumns {
                table_name,
                raw_columns,
            } => dbm.add_columns(&table_name, raw_columns),
            CMD::DropColumn {
                table_name,
                column_name,
            } => dbm.drop_column(&table_name, &column_name),
            CMD::RenameColumn {
                table_name,
                old_name,
                new_name,
            } => dbm.rename_column(&table_name, &old_name, &new_name),
            CMD::ModifyType {
                table_name,
                column_name,
                new_type,
            } => dbm.modify_column_type(&table_name, &column_name, new_type),
            CMD::AddConstraint {
                table_name,
                column_name,
                constraint,
            } => dbm.add_column_constraint(&table_name, &column_name, constraint),
            CMD::DropConstraint {
                table_name,
                column_name,
                constraint,
            } => dbm.drop_column_constraint(&table_name, &column_name, constraint),
            CMD::SetDefault {
                table_name,
                column_name,
                default_value,
            } => dbm.set_default_value(&table_name, &column_name, default_value),

            // row
            CMD::Insert {
                table_name,
                raw_rows,
            } => dbm.insert_rows(&table_name, raw_rows),
            CMD::Update {
                table_name,
                assignments,
                predicate,
            } => dbm.update_rows(&table_name, assignments, predicate),
            CMD::Delete {
                table_name,
                predicate,
            } => dbm.delete_rows(&table_name, predicate),

            // select
            CMD::Select {
                table_name,
                statement,
            } => dbm.select(&table_name, statement),
        };

        if result.is_err() {
            return result;
        }
    }

    Ok(QueryResult::OK)
}
