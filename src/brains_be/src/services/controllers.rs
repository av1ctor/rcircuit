use candid::Principal;
use crate::db::DB;
use neuralarena_cdk::db::traits::crud::{Crud, CrudSubscribable};
use crate::models::controller::{Controller, ControllerId};

pub struct ControllersService {}

impl ControllersService {
    pub fn create(
        ctrl: &Controller,
        db: &mut DB,
        caller: &Principal
    ) -> Result<(), String> {
        if *caller == Principal::anonymous() {
            return Err("Anonymous not allowed".to_string());
        }

        db.controllers.borrow_mut().insert_and_notify(ctrl.id, ctrl.clone())
    }

    pub fn _update(
        id: &ControllerId,
        req: &Controller,
        db: &mut DB,
        caller: &Principal
    ) -> Result<(), String> {
        let mut controllers = db.controllers.borrow_mut();

        let ctrl = match controllers.find_by_id(id) {
            None => return Err("Not found".to_string()),
            Some(e) => e
        };

        if *id != ctrl.created_by || *caller != ctrl.created_by {
            return Err("Forbidden".to_string());
        }

        controllers.update_and_notify(id.to_owned(), req.clone())
    }

    pub fn _delete(
        id: &ControllerId,
        db: &mut DB,
        caller: &Principal
    ) -> Result<(), String> {
        let mut controllers = db.controllers.borrow_mut();

        let ctrl = match controllers.find_by_id(id) {
            None => return Err("Not found".to_string()),
            Some(e) => e
        };

        if *id != ctrl.created_by || *caller != ctrl.created_by {
            return Err("Forbidden".to_string());
        }
        
        controllers.delete_and_notify(id)
    }

    pub fn find_by_id(
        id: &ControllerId,
        db: &DB,
        _caller: &Principal
    ) -> Result<Controller, String> {
        let controllers = db.controllers.borrow();

        let ctrl = match controllers.find_by_id(id) {
            None => return Err("Not found".to_string()),
            Some(e) => e
        };

        Ok((*ctrl).clone())
    }
}