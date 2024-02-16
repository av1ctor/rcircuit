use std::cell::RefCell;
use crate::db::DB;
use brains_engine::consts;
use candid::{CandidType, Principal};
use ic_cdk::{api::stable, caller, trap};
use models::controller::{Controller, ControllerRequest};
use neuralarena_cdk::{
    db::traits::crud::Pagination, 
    models::{
        game::{GameInfo, GameInfoResponse},
        instance::{
            InstanceAssetsResponse, InstanceId, 
            InstanceOptions, InstanceStepsResponse
        }
    },
    utils::{
        random, serdeser, uniqueid
    }
};
use serde::{Deserialize, Serialize};
use services::{
    controllers::ControllersService, 
    instances::InstancesService
};

mod models;
mod db;
mod services;
mod renderers;

const STATE_VERSION: f32 = 0.1;

#[derive(Default, Serialize, Deserialize)]
struct State {
    owner: Option<Principal>,
    asset_canister_url: String,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::default();
    static DB: RefCell<DB> = RefCell::new(DB::new());    
}

#[derive(Clone, CandidType, Deserialize)]
struct InitArg {
    asset_canister_url: String,
    controllers: Vec<ControllerRequest>,
}

#[ic_cdk::init]
fn init(
    arg: InitArg
) {
    ic_cdk::setup();

    STATE.with(|r| {
        let mut state = r.borrow_mut();
        state.owner = Some(caller());
        state.asset_canister_url = arg.asset_canister_url.clone();
    });

    random::init();

    if let Err(msg) = init_controllers(arg.controllers) {
        panic!("Calling init_controllers() failed: {}", msg);
    }
}

fn init_controllers(
    controllers: Vec<ControllerRequest>
) -> Result<(), String> {
    let caller = caller();
        
    DB.with(|db| {
        for controller in controllers {
            let ctrl = Controller::new(&controller, &caller);
            ControllersService::create(&ctrl, &mut db.borrow_mut(), &caller)?;
        }
        Ok(())
    })
}

#[ic_cdk::pre_upgrade]
fn pre_upgrade() {
    let mut writter = stable::StableWriter::default();
    
    STATE.with(|cell| {
        if let Err(err) = serdeser::serialize(
            cell.to_owned(),
            STATE_VERSION,
            &mut writter
        ) {
            trap(&format!(
                "An error occurred when saving state to stable memory (pre_upgrade): {:?}",
                err
            ));
        };
    });

    DB.with(|db| {
        if let Err(err) = db.borrow().serialize(&mut writter) {
            trap(&format!(
                "An error occurred when saving DB to stable memory (pre_upgrade): {:?}",
                err
            ));
        }
    });
}

#[ic_cdk::post_upgrade]
fn post_upgrade() {
    let mut reader = stable::StableReader::default();

    STATE.with(|cell| {
        match serdeser::deserialize::<State>(
            STATE_VERSION, 
            &mut reader
        ) {
            Ok(s_) => {
                cell.replace(s_);
            }
            Err(err) => {
                trap(&format!(
                    "An error occurred when restoring state from stable memory (post_upgrade): {:?}",
                    err
                ));
            }
        }
    });

    DB.with(|db| {
        if let Err(err) = db.borrow_mut().deserialize(&mut reader) {
            trap(&format!(
                "An error occurred when restoring DB from stable memory (post_upgrade): {:?}",
                err
            ));
        }
    });
    
    random::init();
}

#[ic_cdk::query]
pub fn game_get_info(
) -> Result<GameInfoResponse, String> {
    let info = STATE.with(|state| {
        let asset_canister_url = &state.borrow().asset_canister_url;
        GameInfo {
            name: "Braaains".to_string(),
            version: 0.1,
            description: "Eat brains and win the game!".to_string(),
            category: "Zombies".to_string(),
            min_players: consts::MIN_PLAYERS, 
            max_players: consts::MAX_PLAYERS,
            assets_url: asset_canister_url.clone(),
            engine_wasm_url: asset_canister_url.replace("{}", "wasm/game-engine_bg.wasm"),
            image_url: "https://www.digitaltrends.com/wp-content/uploads/2021/03/best-zombie-games-for-xbox-one.jpg?p=1".to_string(),
        }
    });

    Ok(GameInfoResponse{
        info
    })
}

#[ic_cdk::update]
pub fn instance_create(
    options: InstanceOptions
) -> Result<InstanceId, String> {
    let caller = caller();

    DB.with(|db| {
        let id = uniqueid::generate();
        match InstancesService::create(
            &id, 
            options,
            &mut db.borrow_mut(), 
            &caller
        ) {
            Ok(()) => Ok(id),
            Err(msg) => Err(msg)
        }
    })
}

#[ic_cdk::update]        
pub fn instance_start(
    id: InstanceId,
    players: Vec<Principal>
) -> Result<(), String> {     
    DB.with(|db| {
        InstancesService::start(
            id, 
            players,
            &mut db.borrow_mut(), 
            &caller()
        )
    })
}

#[ic_cdk::update]
pub fn instance_run(
    id: InstanceId
) -> Result<(), String> {
    DB.with(|db| {
        random::get().with(|rng| {
            InstancesService::run(
                &id, 
                &mut db.borrow_mut(), 
                &mut rng.borrow_mut().as_mut().unwrap(), 
                &caller()
            )
        })
    })
}

#[ic_cdk::query]
pub fn instance_get_assets(
    id: InstanceId
) -> Result<InstanceAssetsResponse, String> {
    STATE.with(|state| {
        DB.with(|db| {
            match InstancesService::get_assets(
                &db.borrow(), 
                id, 
                &state.borrow().asset_canister_url
            ) {
                Ok(assets) => Ok(InstanceAssetsResponse {
                    assets
                }),
                Err(msg) => Err(msg)
            }
        })
    })
}

#[ic_cdk::query]
pub fn instance_get_steps(
    id: InstanceId,
    pag: Pagination
) -> Result<InstanceStepsResponse, String> {
    DB.with(|db| {
        match InstancesService::get_steps(
            &id, pag, &mut db.borrow_mut(), &caller()
        ) {
            Ok(steps) => Ok(InstanceStepsResponse {
                steps
            }),
            Err(msg) => Err(msg)
        }
    })
}

ic_cdk::export_candid!();