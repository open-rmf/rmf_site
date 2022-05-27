use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone)]
pub struct AgentGroup {
    pub agents_name: Vec<String>,
    pub agents_number: usize,
    pub group_id: usize,
    pub profile_selector: String,
    pub state_selector: String,
    pub x: f64,
    pub y: f64,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct AgentProfile {
    #[serde(rename = "ORCA_tau")]
    pub orca_tau: f64,
    #[serde(rename = "ORCA_tauObst")]
    pub orca_tau_obst: f64,
    pub class: usize,
    pub max_accel: f64,
    pub max_angle_vel: f64,
    pub max_neighbors: usize,
    pub max_speed: f64,
    pub name: String,
    pub neighbor_dist: f64,
    pub obstacle_set: usize,
    pub pref_speed: f64,
    pub r: f64,
}

// TODO:
#[derive(Deserialize, Serialize, Clone)]
pub struct GoalSet;

// TODO:
#[derive(Deserialize, Serialize, Clone)]
pub struct ModelType;

#[derive(Deserialize, Serialize, Clone)]
pub struct ObstacleSet {
    pub class: usize,
    pub file_name: String,
    #[serde(rename = "type")]
    pub type_: String,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct State {
    #[serde(rename = "final")]
    pub final_: usize,
    pub goal_set: i64,
    pub name: String,
    pub navmesh_file_name: String,
}

// TODO:
#[derive(Deserialize, Serialize, Clone)]
pub struct Transition;

#[derive(Deserialize, Serialize)]
pub struct CrowdSim {
    agent_groups: Vec<AgentGroup>,
    agent_profiles: Vec<AgentProfile>,
    enable: i8,
    goal_sets: Vec<GoalSet>,
    model_types: Vec<ModelType>,
    obstacle_set: ObstacleSet,
    states: Vec<State>,
    transitions: Vec<Transition>,
    update_time_step: f64,
}
