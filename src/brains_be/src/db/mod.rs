pub mod tables;
pub mod migrations;

use std::cell::RefCell;
use std::rc::Rc;
use ic_cdk::api::stable::{StableWriter, StableReader};
use self::tables::{controllers::ControllersTable, instances::InstancesTable};
use neuralarena_cdk::db::traits::table::{TableSerializable, TableDeserializable, Table};

#[derive(Clone, Debug)]
pub enum TableName {
    Controllers,
    Instances,
}

pub struct DB {
    pub controllers: Rc<RefCell<ControllersTable>>,
    pub instances: Rc<RefCell<InstancesTable>>,
}

impl DB {
    pub fn new(
    ) -> Self {
        let controllers = Rc::new(RefCell::new(ControllersTable::new()));
        let instances = Rc::new(RefCell::new(InstancesTable::new()));

        Self {
            controllers,
            instances,
        }
    }

    pub fn serialize(
        &self,
        writer: &mut StableWriter
    ) -> Result<(), String> {
        self.controllers.borrow().serialize(writer)?;
        self.instances.borrow().serialize(writer)?;
        Ok(())
    }

    pub fn deserialize(
        &mut self,
        reader: &mut StableReader
    ) -> Result<(), String> {
        self.controllers.borrow_mut().deserialize(reader, true)?;
        self.instances.borrow_mut().deserialize(reader, true)?;
        Ok(())
    }
}
