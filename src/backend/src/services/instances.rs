use std::{collections::{HashMap, HashSet}, f32::consts::PI};
use brains_engine::{
    consts, 
    game::{Game, GameOptions, LevelOptions}, 
    map::Map, 
    player::PlayerFOV
};
use candid::Principal;
use neuralarena_cdk::{
    agent::{agent::Agent, baseagent::BaseAgentOptions, dqnagent::DqnAgent}, 
    db::traits::crud::{Crud, Pagination}, 
    dqn::dqn::DqnOptions, 
    models::{
        assets::{Asset, AssetRef}, env::Env, game_instance::{InstanceId, InstanceStep}
    }, 
    nn::tensor::Tensor,
};
use rand::{rngs::StdRng, Rng};
use crate::{
    models::instance::{Instance, InstanceStatus}, 
    renderers::null_renderer::NullRenderer,
    db::DB
};
use super::controllers::ControllersService;

const TENSORS: &[u8] = include_bytes!("../../tensors/15rays-32hidden-0.05epsilon.safetensors");
const NUM_HIDDEN_LAYERS: usize = 32;
const EPSILON: f64 = 0.05;
const PLAYER_FOV_RAYS: usize = 15;
const PLAYER_FOV_ANGLE: f32 = PI;
const PLAYER_RADIUS: f32 = 0.5;

const MAP: &[u8] = include_bytes!("../../maps/map-no-tex.glb");
const NAVMESH: &[u8] = include_bytes!("../../maps/navmesh.bin");

struct Player {
    active: bool,
    last_thought: u32,
    steps_per_thought: u32,
    obs: Tensor,
}
pub struct InstancesService {}

impl InstancesService {
    pub fn get_assets(
        db: &DB,
        id: InstanceId
    ) -> Result<HashMap<AssetRef, Asset>, String> {
        let instances = db.instances.borrow_mut();

        let instance = match instances.find_by_id(&id) {
            None => return Err("Not found".to_string()),
            Some(e) => e.clone()
        };

        let assets = instance.game.get_assets();
        Ok(assets)
    }

    pub fn create(
        id: &InstanceId,
        options: HashMap<String, String>,
        db: &mut DB,
        caller: &Principal
    ) -> Result<(), String> {
        Self::authorize(caller, db)?;

        let game = Game::new(
            Map::new(MAP, NAVMESH),
            GameOptions {
                level: LevelOptions {
                    num: 0
                },
                fov: PlayerFOV { 
                    rays: PLAYER_FOV_RAYS, 
                    angle: PLAYER_FOV_ANGLE, 
                    max_dist: 2.5,
                },
                min_keys: 4,
                max_steps: 60 * 60 * 2,
                num_players: 0,
                player_radius: PLAYER_RADIUS,
                debug: false,
                training: false,
            }
        );

        
        let instance = Instance::new(
            id, 
            options, 
            game,
            caller
        );
    
        db.instances.borrow_mut().insert(id.clone(), instance)
    }

    pub fn start(
        id: InstanceId,
        players: Vec<Principal>,
        db: &mut DB,
        caller: &Principal
    ) -> Result<(), String> {
        Self::authorize(caller, db)?;
        
        let mut instances = db.instances.borrow_mut();

        let mut instance = match instances.find_by_id(&id) {
            None => return Err("Not found".to_string()),
            Some(e) => e.clone()
        };

        if instance.state.players.len() < consts::MIN_PLAYERS {
            return Err("Too few players".to_string());
        }

        if instance.state.status != InstanceStatus::CREATED {
            return Err("Invalid instance state".to_string());
        }

        // update game
        instance.game.options.num_players = players.len();
        instance.state.players = players;

        // create the agent
        let options = BaseAgentOptions { 
            dqn: DqnOptions {
                n_actions: instance.game.get_num_actions(),
                n_obs: instance.game.get_num_obs(),
                n_hidden: NUM_HIDDEN_LAYERS
            },
            epsilon: EPSILON,
        };
        
        let mut agent = DqnAgent::new(&options);

        /*let tensors_path = format!(
            "./weights/{}rays-{}hidden-{}epsilon.safetensors", 
            instance.options.rays, 
            instance.options.num_hidden_layers, 
            instance.options.fps, 
            instance.options.tps, 
            instance.options.epsilon
        );*/
    
        agent.load(&TENSORS.to_vec())?;

        instance.agent = Some(agent);

        // start match
        instance.state.status = InstanceStatus::STARTED;

        instances.update(id.clone(), instance)
    }

    pub fn run(
        id: &InstanceId,
        db: &mut DB,
        rng: &mut StdRng,
        caller: &Principal
    ) -> Result<(), String> {
        Self::authorize(caller, db)?;
        
        let mut instances = db.instances.borrow_mut();

        let mut instance = match instances.find_by_id(id) {
            None => return Err("Not found".to_string()),
            Some(e) => e.clone()
        };

        if instance.state.players.len() < consts::MIN_PLAYERS {
            return Err("Too few players".to_string());
        }

        if instance.state.status != InstanceStatus::STARTED {
            return Err("Invalid instance state".to_string());
        }

        instance.state.status = InstanceStatus::RUNNING;

        let game = &mut instance.game;
        let mut renderer = NullRenderer::new();
        let agent = instance.agent.as_mut().unwrap();
        
        let mut steps = 0;
        let mut running = true;

        let mut players = vec![];
        let obs = &game.reset(&mut renderer, rng);
        for i in 0..game.options.num_players {
            let steps_per_thought = rng.gen_range(1000..10000);
            players.push(Player {
                active: true,
                last_thought: 0,
                steps_per_thought,
                obs: obs[i].to_owned().into(),
            });
        }

        let pre = ic_cdk::api::performance_counter(0);

        while running && steps < game.options.max_steps {

            let mut actions = vec![None; game.options.num_players];

            let players_idx = Self::shuffle_players_idx(game, rng);
            
            for i in 0..game.options.num_players {
                let p = players_idx[i];
                let player = &mut players[p];
                
                if player.active {
                    player.last_thought += 1000;
                    if player.last_thought >= player.steps_per_thought {
                        player.last_thought = 0;

                        actions[p] = match agent.choose_action(player.obs.clone(), rng) {
                            Some(action) => Some(action),
                            None => Some(game.random_action(rng)),
                        };
                    }
                }
            }

            let step = game.step(&actions, 1.0, &mut renderer, rng);
            steps += 1;

            let entities = game.serialize_entities()?;
            let events = game.serialize_events()?;

            instance.state.steps.push(InstanceStep {
                terminated: step.terminated,
                timedout: step.timedout,
                entities,
                events,
            });

            running = !step.terminated && !step.timedout;

            if let Some(winner) = step.winner {
                instance.won_by = Some(instance.state.players[winner].clone());
                instance.won_at = Some(ic_cdk::api::time());
            }

            for i in 0..step.players.len() {
                players[i].active = step.players[i].active;
                players[i].obs = step.players[i].obs.to_owned().into();
            }

            game.render(&mut renderer, true);
        }

        instance.state.status = InstanceStatus::FINISHED;

        let post = ic_cdk::api::performance_counter(0);
        ic_cdk::print((post - pre).to_string());

        instances.update(id.to_owned(), instance)
    }

    fn shuffle_players_idx(
        game: &mut Game,
        rng: &mut StdRng
    ) -> Vec<usize> {
        let mut indexes = HashSet::new();
    
        while indexes.len() < game.options.num_players {
            indexes.insert(rng.gen_range(0..game.options.num_players));
        }
    
        indexes.into_iter().collect()
    }

    pub fn _delete(
        id: &InstanceId,
        db: &mut DB,
        caller: &Principal
    ) -> Result<(), String> {
        Self::authorize(caller, db)?;
        
        let mut instances = db.instances.borrow_mut();

        let instance = match instances.find_by_id(id) {
            None => return Err("Not found".to_string()),
            Some(e) => e
        };

        if *caller != instance.created_by {
            return Err("Forbidden".to_string());
        }

        if instance.state.status != InstanceStatus::CREATED {
            return Err("Invalid state state".to_string());
        }
        
        instances.delete(id)
    }

    pub fn steps(
        id: &InstanceId,
        pag: Pagination,
        db: &mut DB,
        _caller: &Principal
    ) -> Result<Vec<InstanceStep>, String> {
        let instances = db.instances.borrow_mut();

        let instance = match instances.find_by_id(id) {
            None => return Err("Not found".to_string()),
            Some(e) => e
        };

        if ![InstanceStatus::STARTED, InstanceStatus::FINISHED]
            .contains(&instance.state.status) {
            return Err("Invalid instance state".to_string());
        }

        Ok(instance.state.steps.iter()
            .skip(pag.offset as usize)
            .take(pag.limit as usize)
            .cloned()
            .collect())
    }

    fn authorize(
        caller: &Principal, 
        db: &mut DB
    ) -> Result<(), String> {
        let controller = match ControllersService::find_by_id(caller, db, caller) {
            Ok(e) => e,
            Err(msg) => return Err(msg),
        };

        if !controller.active {
            Err("Controller inactive".to_string())
        }
        else if controller.banned {
            Err("Controller banned".to_string())
        }
        else {
            Ok(())
        }
    }
}