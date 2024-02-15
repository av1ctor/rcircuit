pub mod tables;
pub mod migrations;

use std::cell::RefCell;
use std::rc::Rc;
use ic_cdk::api::stable::{StableWriter, StableReader};
use self::tables::{controllers::ControllersTable, instances::InstancesTable};
use neuralarena_cdk::db::traits::table::{TableSerializable, TableDeserializable, Table};

#[derive(Clone, Debug)]
pub enum TableName {
    Instances,
    Controllers,
}

pub struct DB {
    pub instances: Rc<RefCell<InstancesTable>>,
    pub controllers: Rc<RefCell<ControllersTable>>,
}

impl DB {
    pub fn new(
    ) -> Self {
        let instances = Rc::new(RefCell::new(InstancesTable::new()));
        let controllers = Rc::new(RefCell::new(ControllersTable::new()));

        Self {
            instances,
            controllers,
        }
    }

    pub fn serialize(
        &self,
        writer: &mut StableWriter
    ) -> Result<(), String> {
        self.instances.borrow().serialize(writer)?;
        Ok(())
    }

    pub fn deserialize(
        &mut self,
        reader: &mut StableReader
    ) -> Result<(), String> {
        self.instances.borrow_mut().deserialize(reader, true)?;
        Ok(())
    }
}
