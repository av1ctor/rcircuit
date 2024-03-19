use std::collections::HashMap;
use brains_engine::engine::Game;
use candid::{CandidType, Principal};
use neuralarena_cdk::{
    agent::dqnagent::DqnAgent, 
    models::instance::{InstancePlayer, InstanceRunSlice, InstanceStep}
};
use serde::{Serialize, Deserialize};

#[derive(PartialEq, Clone, Serialize, Deserialize, CandidType)]
pub enum InstanceStatus {
    CREATED,
    STARTED,
    RUNNING,
    CANCELLED,
    FINISHED,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct InstanceState {
    pub status: InstanceStatus,
    pub terminated: bool,
    pub timedout: bool,
    pub players: Vec<InstancePlayer>,
    pub steps: Vec<InstanceStep>,
    pub step: usize,
    pub slices: Vec<InstanceRunSlice>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Instance {
    pub id: String,
    pub options: HashMap<String, String>,
    pub state: InstanceState,
    pub game: Game,
    pub agents: Option<Vec<DqnAgent>>,
    pub created_at: u64,
    pub created_by: Principal,
    pub started_at: Option<u64>,
    pub started_by: Option<Principal>,
    pub won_at: Option<u64>,
    pub won_by: Option<Principal>,
}

impl Instance {
    pub fn new(
        id: &String,
        options: HashMap<String, String>,
        game: Game,
        caller: &Principal
    ) -> Self {
        Self {
            id: id.clone(),
            options, 
            state: InstanceState { 
                status: InstanceStatus::CREATED,
                terminated: false,
                timedout: false,
                players: vec![],
                steps: vec![],
                step: 0,
                slices: vec![],
            },
            game,
            agents: None,
            created_at: ic_cdk::api::time(),
            created_by: caller.clone(),
            started_at: None,
            started_by: None,
            won_at: None,
            won_by: None,
        }
    }
}

