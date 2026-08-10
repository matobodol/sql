use std::{collections::HashMap, sync::Arc};

use crate::{Increment, Schema};

pub struct DB {
    name: String,
    tables: HashMap<String, TBL>,
}

impl DB {
    fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            tables: HashMap::new(),
        }
    }
    fn create(&mut self, tbl_name: &str, schema: Arc<Schema>) {
        let tbl = TBL::new(tbl_name, schema);
        self.tables.insert(tbl_name.to_string(), tbl);
    }
    fn drop(&mut self, tbl_name: &str) {
        self.tables.remove(tbl_name);
    }
    fn rename(&mut self, old_name: &str, new_name: &str) {
        // Mengubah key "old_key" menjadi "new_key"
        if let Some(value) = self.tables.remove(old_name) {
            self.tables.insert(new_name.to_string(), value);
        }
    }
}

pub struct TBL {
    name: String,
    schema: Arc<Schema>,
}

impl TBL {
    pub fn new(name: &str, schema: Arc<Schema>) -> Self {
        let mut auto_increment_counters = HashMap::new();

        for col in schema.columns() {
            if let Some(Increment::Enabled { start, .. }) = col.auto_increment_config() {
                auto_increment_counters.insert(col.id, *start);
            }
        }

        Self {
            name: name.to_string(),
            schema,
        }
    }
}
