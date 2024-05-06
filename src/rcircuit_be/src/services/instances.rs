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
        env::{Env, EnvPlayerOptions}, 
        game::{GameInfo, GameMapId}, 
        instance::{
            InstanceId, InstancePlayer, InstancePlayerEntity, InstancePlayerState, 
            InstanceRunRequest, InstanceRunResponse, InstanceRunSlice, InstanceStep
        }, 
        nas1_types::Nas1TokenGameProps
    }
};
use rand::{rngs::StdRng, Rng, SeedableRng};
use crate::{
    config, 
    db::DB, 
    models::instance::{Instance, InstanceStatus}, 
    renderers::null_renderer::NullRenderer
};
use super::controllers::ControllersService;

const PLAYER_FOV_RAYS: usize = 15;
const PLAYER_FOV_ANGLE: f32 = PI;
const PLAYER_FOV_DISTANCE: f32 = 2.5;

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
                    max_dist: PLAYER_FOV_DISTANCE,
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
            let props = player.nft.props.as_ref().unwrap();
            
            if props.num_hidden_layers != 1 {
                return Err(format!(
                    "Only 1 hidden layer supported. Found: {}", 
                    props.num_hidden_layers
                ));
            }
            
            // create an agent for each skill
            let mut agent_skills = HashMap::new();
            for skill in config::SKILLS {
                let options = BaseAgentOptions { 
                    dqn: DqnOptions {
                        n_actions: inst.game.get_num_actions(&skill),
                        n_obs: inst.game.get_num_obs(&skill),
                        n_hidden: props.hidden_layers_weights_size[0]
                    },
                    epsilon: props.epsilon as _,
                };

                let mut agent = DqnAgent::new(&options);

                agent.load(&player.tensors[*skill])?;

                agent_skills.insert(skill.to_string(), agent);
            }

            agents.push(agent_skills);
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
            let canister_id = ic_cdk::id();
            let players_obs = game.reset(
                &inst.state.players.iter().map(|p| {
                    let props = p.nft.props.as_ref().unwrap();
                    let game_props = props.game_props
                        .get(&canister_id)
                        .or(Some(&Nas1TokenGameProps{
                            xp: 0.0
                        }))
                        .unwrap();
                    EnvPlayerOptions {
                        radius: props.radius,
                        height: props.height,
                        xp: game_props.xp,
                    }
                }).collect(), 
                &mut renderer, 
                rng
            );
            
            let mut i = 0;
            for obs in players_obs {
                inst.state.players[i].state.obs = obs;
                i += 1;
            }
        }

        let begin_t = ic_cdk::api::performance_counter(0);

        let mut players_idx = Self::create_players_idx(game, rng);

        while inst.state.step < req.max_steps {
            let mut actions = vec![(0usize, None); game.options.num_players];

            Self::circ_players_idx(&mut players_idx);
            
            for i in 0..game.options.num_players {
                let p = players_idx[i];
                let player = &mut inst.state.players[p];

                actions[i].0 = p;
                
                if player.state.active {
                    player.state.last_thought += 1000;
                    let props = player.nft.props.as_ref().unwrap();
                    if player.state.last_thought >= props.steps_per_thought {
                        player.state.last_thought = 0;

                        let skill = &player.state.skill;
                        if let Some(agent) = agents[p].get_mut(skill) {
                            actions[i].1 = match agent.choose_action(
                                player.state.obs[skill].clone().into(), rng) {
                                Some(action) => Some(action),
                                None => Some(game.random_action(skill, rng)),
                            };
                        }
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

            let mut i = 0;
            for step_player in step.players {
                let inst_player = &mut inst.state.players[i];
                let game_player = &game.entities.players[i];

                inst_player.state = InstancePlayerState {
                    active: step_player.active,
                    skill: step_player.skill,
                    obs: step_player.obs,
                    score: game_player.num_brains as _,
                    health: game_player.health,
                    xp: game_player.xp,
                    ..inst_player.state
                };

                i += 1;
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
                    score: p.state.score,
                    xp: p.state.xp, 
                })
                .collect(),
            won_at: inst.won_at.clone(),
            won_by: inst.won_by.clone(),
        };

        instances.update(id.to_owned(), inst)?;

        Ok(res)
    }

    fn create_players_idx(
        game: &mut Game,
        rng: &mut StdRng
    ) -> Vec<usize> {
        let mut indexes = HashSet::new();
    
        while indexes.len() < game.options.num_players {
            indexes.insert(rng.gen_range(0..game.options.num_players));
        }
    
        indexes.into_iter().collect()
    }

    fn circ_players_idx(
        indexes: &mut Vec<usize>
    ) {
        let len = indexes.len();
        
        let first = indexes[0];
        for i in 0..len-1 {
            indexes[i] = indexes[i+1];
        }

        indexes[len-1] = first;
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