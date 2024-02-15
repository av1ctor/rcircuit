use std::collections::BTreeMap;
use crate::db::TableName;
use neuralarena_cdk::{
    db::traits::{
        crud::Crud, 
        table::{
            Table, TableData, TableDeserializable, TableSchema, TableSerializable, TableVersioned
        }
    }, 
    models::instance::InstanceId
};
use crate::models::instance::Instance;

pub struct InstancesTable {
    pub schema: TableSchema<TableName>,
    pub data: TableData<InstanceId, Instance>
}

impl Table<TableName, InstanceId, Instance> for InstancesTable {
    fn new(
    ) -> Self {
        Self {
            schema: TableSchema { 
                version: 0.1,
                name: TableName::Instances,
            },
            data: TableData(BTreeMap::new())
        }
    }

    fn get_data(
        &self
    ) -> &TableData<InstanceId, Instance> {
        &self.data
    }

    fn get_data_mut(
        &mut self
    ) -> &mut TableData<InstanceId, Instance> {
        &mut self.data
    }

    fn set_data(
        &mut self,
        data: TableData<InstanceId, Instance>
    ) {
        self.data = data;
    }
    
    fn get_schema(
        &self
    ) -> &TableSchema<TableName> {
        &self.schema
    }
}

impl TableSerializable<TableName, InstanceId, Instance> for InstancesTable {}

impl TableVersioned<TableName, InstanceId, Instance> for InstancesTable {}

impl TableDeserializable<TableName, InstanceId, Instance> for InstancesTable {}

impl Crud<TableName, InstanceId, Instance> for InstancesTable {}
