use candid::{Principal, CandidType};
use serde::{Deserialize, Serialize};

pub type ControllerId = Principal;

#[derive(Clone, Serialize, Deserialize)]
pub struct Controller {
    pub id: ControllerId,
    pub name: String,
    pub description: Option<String>,
    pub active: bool,
    pub banned: bool,
    pub created_at: u64,
    pub created_by: Principal,
    pub updated_at: Option<u64>,
    pub updated_by: Option<Principal>,
    pub deleted_at: Option<u64>,
    pub deleted_by: Option<Principal>,
}

#[derive(CandidType, Deserialize, Clone)]
pub struct ControllerRequest {
    principal: Principal,
    name: String,
    description: Option<String>,
}

#[derive(CandidType)]
pub struct ControllerResponse {
    id: ControllerId,
    name: String,
    description: Option<String>,
    active: bool,
    banned: bool,
    created_at: u64,
    updated_at: Option<u64>,
}

impl Controller {
    pub fn new(
        e: &ControllerRequest,
        caller: &Principal
    ) -> Self {
        Self {
            id: e.principal.clone(),
            name: e.name.clone(),
            description: e.description.clone(),
            active: true,
            banned: false,
            created_at: ic_cdk::api::time(),
            created_by: caller.clone(),
            updated_at: None,
            updated_by: None,
            deleted_at: None,
            deleted_by: None,
        }
    }

    pub fn _update(
        &self,
        caller: &Principal
    ) -> Self {
        Self {
            updated_at: Some(ic_cdk::api::time()),
            updated_by: Some(caller.clone()),
            ..self.clone()
        }
    }
}

impl From<Controller> for ControllerResponse {
    fn from(
        e: Controller
    ) -> Self {
        Self { 
            id: e.id,
            name: e.name,
            description: e.description,
            active: e.active,
            banned: e.banned,
            created_at: e.created_at,
            updated_at: e.updated_at,
        }
    }
}
