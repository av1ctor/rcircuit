use std::{cell::RefCell, collections::HashMap};
use crate::db::DB;
use rcircuit_engine::consts;
use candid::{CandidType, Principal};
use ic_cdk::{api::stable, caller, trap};
use models::controller::{Controller, ControllerRequest};
use neuralarena_cdk::{
    db::traits::crud::Pagination, 
    models::{
        game::{GameCost, GameCosts, GameEndCredits, GameInfo, GameInfoResponse, GameMap, GameURLs},
        instance::{
            InstanceAssetsResponse, InstanceId, InstanceOptions, 
            InstancePlayer, InstanceRunRequest, InstanceRunResponse, 
            InstanceStepsResponse
        },
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

const COST_TO_PLAY: u128 = (0.01 * 1_00000000.0) as _;
const SKILLS: &[&str] = &[
    "seek-target"
];
const ANIMATIONS: &[&str] = &[
    "idle",
    "walking",
    "celebrating",
    "attacking",
    "hit",
    "dying",
    "dead",
];

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
        let scenes = HashMap::from([
            ("start", asset_canister_url.replace("{}", "scenes/start/start.gltf")),
            ("intro", asset_canister_url.replace("{}", "scenes/intro/intro.gltf")),
        ]).iter().map(|e| (e.0.to_string(), e.1.to_string())).collect();
        
        GameInfo {
            name: "Rusty Circuit".to_string(),
            version: 0.1,
            description: "Try to beat your enemies - and the tetanus!".to_string(),
            category: "Post apocalyptic".to_string(),
            fps: consts::FPS as _,
            min_players: consts::MIN_PLAYERS, 
            max_players: consts::MAX_PLAYERS,
            costs: GameCosts { 
                to_play: GameCost::ICP(COST_TO_PLAY)
            },
            urls: GameURLs{
                 cover: asset_canister_url.replace("{}", "cover.jpg"),
                 engine_wasm: asset_canister_url.replace("{}", "wasm/game-engine_bg.wasm"),
                 assets: asset_canister_url.clone(),
                 scenes
            },
            skills: SKILLS.iter().map(|s| s.to_string()).collect(),
            animations: ANIMATIONS.iter().map(|a| a.to_string()).collect(),
            maps: vec![
                GameMap { 
                    id: consts::MAP_ID_CONTAINERS, 
                    name: "Containers".to_string(), 
                    description: "A labirynth of containers".to_string(), 
                    version: 0.1, 
                    image_url: asset_canister_url.replace("{}", &format!("maps/{}/cover.jpg", consts::MAP_ID_CONTAINERS)),
                }
            ],
        }
    });

    Ok(GameInfoResponse{
        info
    })
}

#[ic_cdk::query]
pub fn game_get_end_credits(
) -> Result<GameEndCredits, String> {
    Ok(GameEndCredits {
        title: "Rusty Circuit".to_string(),
        people: vec![],
        sub: vec![
            GameEndCredits { 
                title: "💻 Programming 💻".to_string(), 
                people: vec![
                    ("Main Developer".to_string(), "André Vicentini".to_string()),
                    ("Additional dev".to_string(), "Me, Again".to_string()),
                ], 
                sub: vec![] 
            },
            GameEndCredits { 
                title: "🧸 Graphics 🧸".to_string(), 
                people: vec![
                    ("3D Modeler".to_string(), "André Vicentini".to_string())
                ], 
                sub: vec![] 
            },
            GameEndCredits { 
                title: "🎆 Game Design 🎆".to_string(), 
                people: vec![
                    ("Main".to_string(), "André Vicentini".to_string())
                ], 
                sub: vec![] 
            },
            GameEndCredits { 
                title: "🎶 Music 🎶".to_string(), 
                people: vec![
                    ("Composer".to_string(), "Unknown".to_string())
                ], 
                sub: vec![] 
            },
            GameEndCredits { 
                title: "🧨 Sound Effects 🧨".to_string(), 
                people: vec![
                    ("Composer".to_string(), "Unknown".to_string())
                ], 
                sub: vec![] 
            },
            GameEndCredits { 
                title: "🔬 Quality Assurance 🔬".to_string(), 
                people: vec![
                    ("Director".to_string(), "Unknown".to_string()),
                    ("Tester".to_string(), "Unknown".to_string())
                ], 
                sub: vec![] 
            },
            GameEndCredits { 
                title: "🎉 Special Thanks 🎉".to_string(), 
                people: vec![
                    ("Foo foo".to_string(), "Bar bar".to_string()),
                    ("Bar bar".to_string(), "Baz Baz".to_string()),
                    ("Baz baz".to_string(), "Foo foo".to_string()),
                ], 
                sub: vec![] 
            }
        ],
    })
}

#[ic_cdk::update]
pub fn instance_create(
    map_id: usize,
    options: InstanceOptions
) -> Result<InstanceId, String> {
    let caller = caller();

    DB.with(|db| {
        let id = uniqueid::generate();
        match InstancesService::create(
            &id, 
            map_id,
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
pub fn instance_setup(
    id: InstanceId,
    players: Vec<InstancePlayer>
) -> Result<(), String> {     
    DB.with(|db| {
        InstancesService::setup(
            id, 
            players,
            &mut db.borrow_mut(), 
            &caller()
        )
    })
}

#[ic_cdk::update]
pub fn instance_run(
    id: InstanceId,
    req: InstanceRunRequest
) -> Result<InstanceRunResponse, String> {
    DB.with(|db| {
        InstancesService::run(
            &id,
            &req, 
            &mut db.borrow_mut(), 
            &caller()
        )
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