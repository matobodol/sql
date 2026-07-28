use crate::domain::{ColumnDef, DomainError, Row, Schema, SqlType, Table};
use crate::execution::{
    CreateTableOperator, FilterOperator, InsertOperator, LimitOperator, OrderByExpr,
    PhysicalOperator, ProjectionOperator, SeqScanOperator, SortOperator, SortOrder,
};
use crate::planner::expr_mapper::map_expr;

use sqlparser::ast::{
    ColumnDef as SqlColumnDef, CreateTable, DataType, Expr as SqlExpr, Insert, ObjectName, Query,
    Select, SelectItem, SetExpr, Statement, TableFactor, TableObject, Value,
};
use sqlparser::dialect::GenericDialect;
use sqlparser::parser::Parser;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Catalog sebagai kamus/peta yang mengelola banyak `Table`.
#[derive(Debug, Clone, Default)]
pub struct Catalog {
    tables: Arc<RwLock<HashMap<String, Table>>>,
}

impl Catalog {
    pub fn new() -> Self {
        Self::default()
    }

    /// Mendaftarkan objek `Table` baru ke catalog
    pub fn register_table(&self, table: Table) -> Result<(), DomainError> {
        let mut tables = self.tables.write().map_err(|_| {
            DomainError::EvaluationError("Gagal mendapatkan write lock pada catalog".into())
        })?;

        if tables.contains_key(table.name()) {
            return Err(DomainError::EvaluationError(format!(
                "Tabel '{}' sudah ada di dalam catalog",
                table.name()
            )));
        }

        tables.insert(table.name().to_string(), table);
        Ok(())
    }

    /// Mengambil objek `Table` berdasarkan nama
    pub fn get_table(&self, name: &str) -> Result<Table, DomainError> {
        let tables = self.tables.read().map_err(|_| {
            DomainError::EvaluationError("Gagal mendapatkan read lock pada catalog".into())
        })?;

        tables.get(name).cloned().ok_or_else(|| {
            DomainError::EvaluationError(format!("Tabel '{name}' tidak ditemukan di catalog"))
        })
    }
}

/// QueryPlanner bertanggung jawab mengubah Query String / AST menjadi Rantai `PhysicalOperator`
pub struct QueryPlanner<'a> {
    catalog: &'a Catalog,
}

impl<'a> QueryPlanner<'a> {
    pub fn new(catalog: &'a Catalog) -> Self {
        Self { catalog }
    }

    /// Entry point utama untuk membangun physical execution plan dari SQL String
    pub fn build_plan(&self, sql: &str) -> Result<Box<dyn PhysicalOperator>, DomainError> {
        let dialect = GenericDialect {};
        let statements = Parser::parse_sql(&dialect, sql)
            .map_err(|e| DomainError::InvalidExpression(e.to_string()))?;

        let statement = statements
            .first()
            .ok_or_else(|| DomainError::InvalidExpression("Query kosong".into()))?;

        match statement {
            Statement::Query(query) => self.plan_query(query),

            Statement::CreateTable(CreateTable { name, columns, .. }) => {
                self.plan_create_table(name, columns)
            }

            // Perbaikan: gunakan field `table` (bukan `table_name`)
            Statement::Insert(Insert { table, source, .. }) => self.plan_insert(table, source),

            _ => Err(DomainError::InvalidExpression(
                "Statement SQL belum didukung".into(),
            )),
        }
    }

    /// Membangun plan untuk klausa `SELECT`
    fn plan_query(&self, query: &Query) -> Result<Box<dyn PhysicalOperator>, DomainError> {
        let select = match &*query.body {
            SetExpr::Select(select) => select,
            _ => {
                return Err(DomainError::InvalidExpression(
                    "Set expression belum didukung".into(),
                ));
            }
        };

        // 1. Plan Table Scan dari objek `Table`
        let table_name = extract_table_name(select)?;
        let table = self.catalog.get_table(&table_name)?;

        let schema = table.schema().clone();
        let rows = table.scan()?;

        let mut plan: Box<dyn PhysicalOperator> = Box::new(SeqScanOperator::new(schema, rows));

        // 2. Plan Filter (`WHERE`)
        if let Some(selection) = &select.selection {
            let predicate = map_expr(selection)?;
            plan = Box::new(FilterOperator::new(plan, predicate));
        }

        // 3. Plan Sort (`ORDER BY`)
        if let Some(order_by_clause) = &query.order_by {
            let mut order_exprs = Vec::new();
            for order_item in &order_by_clause.exprs {
                let expr = map_expr(&order_item.expr)?;
                let order = match order_item.asc {
                    Some(false) => SortOrder::Descending,
                    _ => SortOrder::Ascending,
                };
                order_exprs.push(OrderByExpr { expr, order });
            }
            plan = Box::new(SortOperator::new(plan, order_exprs));
        }

        // 4. Plan Limit & Offset (`LIMIT`)
        if query.limit.is_some() || query.offset.is_some() {
            let limit = query.limit.as_ref().and_then(parse_limit_expr);
            let offset = query
                .offset
                .as_ref()
                .and_then(|o| parse_limit_expr(&o.value))
                .unwrap_or(0);

            plan = Box::new(LimitOperator::new(plan, limit, offset));
        }

        // 5. Plan Projection (`SELECT col1, col2`)
        plan = self.plan_projection(plan, &select.projection)?;

        Ok(plan)
    }

    /// Membangun plan untuk `CREATE TABLE`
    fn plan_create_table(
        &self,
        name: &ObjectName,
        sql_columns: &[SqlColumnDef],
    ) -> Result<Box<dyn PhysicalOperator>, DomainError> {
        let table_name = name
            .0
            .last()
            .map(|id| id.value.clone())
            .ok_or_else(|| DomainError::InvalidExpression("Nama tabel tidak valid".into()))?;

        let mut columns = Vec::new();

        for col in sql_columns {
            let col_name = col.name.value.clone();
            let sql_type = match &col.data_type {
                // 1. Primitive Types
                DataType::Int(_)
                | DataType::Integer(_)
                | DataType::BigInt(_)
                | DataType::SmallInt(_) => SqlType::Int,
                DataType::Float(_)
                | DataType::Double(_)
                | DataType::Real
                | DataType::Decimal(_) => SqlType::Float,
                DataType::Text | DataType::Varchar(_) | DataType::Char(_) | DataType::String(_) => {
                    SqlType::Text
                }
                DataType::Boolean => SqlType::Bool,
                DataType::Bytea
                | DataType::Binary(_)
                | DataType::Varbinary(_)
                | DataType::Blob(_) => SqlType::Bytes,

                // 2. Date and Time
                DataType::Timestamp(..) => SqlType::Timestamp,
                DataType::Date => SqlType::Date,
                DataType::Time(..) => SqlType::Time,

                // 3. Custom / Fallback
                DataType::Custom(name, _) => {
                    let custom_name = name.to_string();
                    SqlType::Custom(custom_name)
                }
                other => SqlType::Custom(other.to_string()),
            };

            columns.push(ColumnDef::new(col_name, sql_type, true));
        }

        let schema = Schema::new(columns);
        let table = Table::new(table_name, schema);

        Ok(Box::new(CreateTableOperator::new(
            self.catalog.clone(),
            table,
        )))
    }

    /// Membangun plan untuk `INSERT INTO`
    fn plan_insert(
        &self,
        table_object: &TableObject,
        source: &Option<Box<Query>>,
    ) -> Result<Box<dyn PhysicalOperator>, DomainError> {
        // Match TableObject enum untuk mendapatkan ObjectName
        let object_name = match table_object {
            TableObject::TableName(name) => name,
            _ => {
                return Err(DomainError::InvalidExpression(
                    "Tipe target tabel INSERT tidak didukung".into(),
                ));
            }
        };

        // Ekstrak nama tabel dari ObjectName
        let table_name = object_name
            .0
            .last()
            .map(|id| id.value.clone())
            .ok_or_else(|| DomainError::InvalidExpression("Nama tabel tidak valid".into()))?;

        let query = source.as_ref().ok_or_else(|| {
            DomainError::InvalidExpression("VALUES dibutuhkan untuk INSERT".into())
        })?;

        let mut rows = Vec::new();
        if let SetExpr::Values(values) = &*query.body {
            for row_exprs in &values.rows {
                let mut row_values = Vec::new();
                for expr_ast in row_exprs {
                    let expr = map_expr(expr_ast)?;
                    let val = expr.eval(&Schema::default(), &Row::new(vec![]))?;
                    row_values.push(val);
                }
                rows.push(Row::new(row_values));
            }
        }

        let table = self.catalog.get_table(&table_name)?;

        Ok(Box::new(InsertOperator::new(table, rows)))
    }

    /// Helper internal untuk memetakan proyeksi klausa SELECT
    fn plan_projection(
        &self,
        input: Box<dyn PhysicalOperator>,
        projection_ast: &[SelectItem],
    ) -> Result<Box<dyn PhysicalOperator>, DomainError> {
        let input_schema = input.schema();
        let mut exprs = Vec::new();
        let mut output_cols = Vec::new();

        for item in projection_ast {
            match item {
                SelectItem::UnnamedExpr(expr_ast) => {
                    let expr = map_expr(expr_ast)?;
                    let col_name = match &expr {
                        crate::domain::Expr::Column(name) => name.clone(),
                        _ => format!("{expr_ast}"),
                    };

                    let sql_type = if let crate::domain::Expr::Column(name) = &expr {
                        input_schema
                            .columns()
                            .iter()
                            .find(|c| c.name == *name)
                            .map(|c| c.sql_type.clone())
                            .unwrap_or(SqlType::Text)
                    } else {
                        SqlType::Text
                    };

                    output_cols.push(ColumnDef::new(col_name, sql_type, true));
                    exprs.push(expr);
                }
                SelectItem::Wildcard(_) => {
                    for col in input_schema.columns() {
                        output_cols.push(col.clone());
                        exprs.push(crate::domain::Expr::Column(col.name.clone()));
                    }
                }
                _ => {
                    return Err(DomainError::InvalidExpression(
                        "Item projection belum didukung".into(),
                    ));
                }
            }
        }

        let output_schema = Schema::new(output_cols);
        Ok(Box::new(ProjectionOperator::new(
            input,
            exprs,
            output_schema,
        )))
    }
}

fn extract_table_name(select: &Select) -> Result<String, DomainError> {
    let table = select
        .from
        .first()
        .ok_or_else(|| DomainError::InvalidExpression("Klausa FROM dibutuhkan".into()))?;

    match &table.relation {
        TableFactor::Table { name, .. } => {
            Ok(name.0.last().map(|id| id.value.clone()).unwrap_or_default())
        }
        _ => Err(DomainError::InvalidExpression(
            "Tipe tabel tidak didukung".into(),
        )),
    }
}

fn parse_limit_expr(expr: &SqlExpr) -> Option<usize> {
    if let SqlExpr::Value(Value::Number(n, _)) = expr {
        n.parse::<usize>().ok()
    } else {
        None
    }
}
