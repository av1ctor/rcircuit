use std::collections::BTreeMap;
use crate::db::TableName;
use neuralarena_cdk::db::traits::{
    table::{
        TableSerializable, TableDeserializable, TableData, Table, TableSchema, 
        TableVersioned, TableSubscribable, TableSubs, TableEventKey
    },
    crud::{CrudSubscribable, Crud}
};
use crate::models::controller::{ControllerId, Controller};

pub struct ControllersTable {
    pub schema: TableSchema<TableName>,
    pub data: TableData<ControllerId, Controller>,
    pub subs: TableSubs<TableName>,
}

impl Table<TableName, ControllerId, Controller> for ControllersTable {
    fn new(
    ) -> Self {
        Self {
            schema: TableSchema { 
                version: 0.1,
                name: TableName::Controllers,
            },
            data: TableData(BTreeMap::new()),
            subs: TableSubs(Vec::new()),
        }
    }

    fn get_data(
        &self
    ) -> &TableData<ControllerId, Controller> {
        &self.data
    }

    fn get_data_mut(
        &mut self
    ) -> &mut TableData<ControllerId, Controller> {
        &mut self.data
    }

    fn set_data(
        &mut self,
        data: TableData<ControllerId, Controller>
    ) {
        self.data = data;
    }
    
    fn get_schema(
        &self
    ) -> &TableSchema<TableName> {
        &self.schema
    }
}

impl TableSerializable<TableName, ControllerId, Controller> for ControllersTable {}

impl TableVersioned<TableName, ControllerId, Controller> for ControllersTable {}

impl TableDeserializable<TableName, ControllerId, Controller> for ControllersTable {}

impl Crud<TableName, ControllerId, Controller> for ControllersTable {}

impl CrudSubscribable<TableName, ControllerId, Controller> for ControllersTable {}

impl TableSubscribable<TableName, ControllerId, Controller> for ControllersTable {
    fn get_subs(
        &self
    ) -> &TableSubs<TableName> {
        &self.subs
    }

    fn get_subs_mut(
        &mut self
    ) -> &mut TableSubs<TableName> {
        &mut self.subs
    }

    fn get_pkey(
        k: &ControllerId
    ) -> TableEventKey {
        TableEventKey::Principal(k.clone())
    }

    fn get_keys(
        _v: &Controller
    ) -> Vec<TableEventKey> {
        vec![
        ]
    }
}