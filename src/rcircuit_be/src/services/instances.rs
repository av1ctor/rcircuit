use std::{collections::{HashMap, HashSet}, f32::consts::PI};
use rcircuit_engine::{
    consts::{self, FPS}, 
    engine::{Game, GameOptions, MapInfo, MAPS}, 
    entities::PlayerFOV, 
    map::Map
};
use candid::Principal;
use neuralarena_cdk::{
    agent::{
        agent::Agent, baseagent::BaseAgentOptions, 
        dqnagent::DqnAgent
    }, 
    db::traits::crud::{Crud, Pagination}, 
    dqn::dqn::DqnOptions, 
    models::{
        assets::{Asset, AssetRef}, 
        env::Env, 
        game::{GameInfo, GameMapId}, 
        instance::{
            InstanceId, InstancePlayer, InstancePlayerEntity, 
            InstanceRunRequest, InstanceRunResponse, InstanceRunSlice, 
            InstanceStep
        }
    }
};
use rand::{rngs::StdRng, Rng, SeedableRng};
use crate::{
    models::instance::{Instance, InstanceStatus}, 
    renderers::null_renderer::NullRenderer,
    db::DB
};
use super::controllers::ControllersService;

const PLAYER_FOV_RAYS: usize = 15;
const PLAYER_FOV_ANGLE: f32 = PI;

pub struct InstancesService {}

impl InstancesService {
    pub fn get_assets(
        db: &DB,
        id: InstanceId,
        game: &GameInfo
    ) -> Result<HashMap<AssetRef, Asset>, String> {
        let instances = db.instances.borrow_mut();

        let instance = match instances.find_by_id(&id) {
            None => return Err("Not found".to_string()),
            Some(e) => e.clone()
        };

        Ok(instance.game.get_assets(
            Some(game),
            instance.game.map_info.map_id,
            &instance.state.players.iter().map(|p| Some(p)).collect()
        ))
    }

    pub fn create(
        id: &InstanceId,
        map_id: GameMapId,
        options: HashMap<String, String>,
        db: &mut DB,
        caller: &Principal
    ) -> Result<(), String> {
        Self::authorize(caller, db)?;

        let map = if map_id < consts::NUM_MAPS {
            Map::new(
                MAPS[map_id].map, 
                MAPS[map_id].navmesh,
            )
        }
        else {
            return Err(format!("Unknown map {}", map_id));
        };

        let game = Game::new(
            map,
            MapInfo {
                map_id,
                ..Default::default()
            },
            GameOptions {
                fov: PlayerFOV { 
                    rays: PLAYER_FOV_RAYS, 
                    angle: PLAYER_FOV_ANGLE, 
                    max_dist: 2.5,
                },
                min_keys: 4,
                num_players: 0,
                debug: false,
                training: false,
            }
        );
        
        let inst = Instance::new(
            id, 
            options, 
            game,
            caller
        );
    
        db.instances.borrow_mut().insert(id.clone(), inst)
    }

    pub fn setup(
        id: InstanceId,
        players: Vec<InstancePlayer>,
        db: &mut DB,
        caller: &Principal
    ) -> Result<(), String> {
        Self::authorize(caller, db)?;
        
        let mut instances = db.instances.borrow_mut();

        let mut inst = match instances.find_by_id(&id) {
            None => return Err("Not found".to_string()),
            Some(e) => e.clone()
        };

        if players.len() < consts::MIN_PLAYERS {
            return Err("Too few players".to_string());
        }

        if inst.state.status != InstanceStatus::CREATED {
            return Err("Invalid instance state".to_string());
        }

        // update game
        inst.game.options.num_players = players.len();
        inst.state.players = players;

        let mut agents = vec![];

        for player in &inst.state.players {
            if player.nft.props.num_hidden_layers != 1 {
                return Err(format!(
                    "Only 1 hidden layer supported. Found: {}", 
                    player.nft.props.num_hidden_layers
                ));
            }
            
            // create the agent
            let options = BaseAgentOptions { 
                dqn: DqnOptions {
                    n_actions: inst.game.get_num_actions(),
                    n_obs: inst.game.get_num_obs(),
                    n_hidden: player.nft.props.hidden_layers_weights_size[0]
                },
                epsilon: player.nft.props.epsilon as _,
            };

            let mut agent = DqnAgent::new(&options);

            agent.load(&player.tensors["seek-target"])?;

            agents.push(agent);
        }

        inst.agents = Some(agents);

        // start match
        inst.state.status = InstanceStatus::RUNNING;

        instances.update(id.clone(), inst)
    }

    pub fn run(
        id: &InstanceId,
        req: &InstanceRunRequest,
        db: &mut DB,
        caller: &Principal
    ) -> Result<InstanceRunResponse, String> {
        Self::authorize(caller, db)?;
        
        let mut instances = db.instances.borrow_mut();

        let mut inst = match instances.find_by_id(id) {
            None => return Err("Not found".to_string()),
            Some(e) => e.clone()
        };

        if inst.state.players.len() < consts::MIN_PLAYERS {
            return Err("Too few players".to_string());
        }

        if inst.state.status != InstanceStatus::RUNNING {
            return Err("Invalid instance state".to_string());
        }

        inst.state.slices.push(InstanceRunSlice {
            first_step: inst.state.step,
            seed: req.seed,
        });

        inst.state.status = InstanceStatus::RUNNING;

        let rng = &mut StdRng::seed_from_u64(req.seed);

        let game = &mut inst.game;
        let mut renderer = NullRenderer::new();
        let agents = inst.agents.as_mut().unwrap();
        
        if inst.state.step == 0 {
            let obs = &game.reset(&inst.state.players, &mut renderer, rng);
            for (i, player) in &mut inst.state.players.iter_mut().enumerate() {
                player.state.obs = obs[i].clone();
            }
        }

        let begin_t = ic_cdk::api::performance_counter(0);

        while inst.state.step < req.max_steps {
            let mut actions = vec![(0usize, None); game.options.num_players];

            let players_idx = Self::shuffle_players_idx(game, rng);
            
            for i in 0..game.options.num_players {
                let p = players_idx[i];
                let player = &mut inst.state.players[p];

                actions[i].0 = p;
                
                if player.state.active {
                    player.state.last_thought += 1000;
                    if player.state.last_thought >= player.nft.props.steps_per_thought {
                        player.state.last_thought = 0;

                        actions[i].1 = match agents[p].choose_action(
                            player.state.obs.clone().into(), rng) {
                            Some(action) => Some(action),
                            None => Some(game.random_action(rng)),
                        };
                    }
                }
            }

            let step = game.step(&actions, 1.0, &mut renderer, rng);

            let entities = game.serialize_entities()?;
            let events = game.serialize_events()?;

            inst.state.terminated = step.terminated;

            inst.state.steps.push(InstanceStep {
                entities,
                events,
            });

            if let Some(winner) = step.winner {
                inst.won_by = Some(inst.state.players[winner].id.clone());
                inst.won_at = Some(ic_cdk::api::time());
                inst.state.terminated = true;
            }

            for (i, player) in step.players.iter().enumerate() {
                let inst_player = &mut inst.state.players[i];

                inst_player.state.active = player.active;
                inst_player.state.obs = player.obs.clone();
                inst_player.state.score = game.entities.players[i].num_brains as _;
                inst_player.state.health = game.entities.players[i].health;
            }

            game.render(&mut renderer, 1000.0 / FPS as f32);

            if inst.state.terminated || inst.state.timedout {
                break;
            }

            if ic_cdk::api::performance_counter(0) - begin_t >= req.max_instructions {
                break;
            }

            inst.state.step += 1;
        }

        if inst.state.step >= req.max_steps {
            inst.state.timedout = true;
        }
        
        if inst.state.terminated || 
            inst.state.timedout || 
            inst.won_by.is_some() {
            inst.state.status = InstanceStatus::FINISHED;
        }

        let end_t = ic_cdk::api::performance_counter(0);
        ic_cdk::print((end_t - begin_t).to_string());

        let res = InstanceRunResponse {
            terminated: inst.state.terminated,
            timedout: inst.state.timedout,
            steps: inst.state.step,
            players: inst.state.players.iter()
                .map(|p| InstancePlayerEntity {
                    health: p.state.health, 
                    score: p.state.score
                })
                .collect(),
            won_at: inst.won_at.clone(),
            won_by: inst.won_by.clone(),
        };

        instances.update(id.to_owned(), inst)?;

        Ok(res)
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

    pub fn get_steps(
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

        if ![InstanceStatus::RUNNING, InstanceStatus::FINISHED]
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
        let controller = ControllersService::find_by_id(caller, db, caller)?;

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