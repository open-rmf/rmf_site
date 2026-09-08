//! A simple discrete event simulation of of client reqeuests being served by a single server.

use bevy::prelude::*;
use rand::Rng;
use rand::SeedableRng;
use rand::rngs::StdRng;
use rmf_site_sim::event::{CandidateComponentEventWriter, CandidateEventWriter};
use rmf_site_sim::time::{SimulationClock, SimulationTime};
use rmf_site_sim::{Simulation, SimulationBuilder, SimulationComputeState, SimulationPlugin};
use std::time::Duration;

/// Seed randomness for reproducibility.
const RANDOMNESS_SEED: u64 = 12345;

const NUM_CLIENT_REQUESTS: u64 = 3;
const MAX_ARRIVAL_TIME: f32 = 10.0;
const SERVER_RESPONSE_TIME: Duration = Duration::from_secs(5);

/// A marker component for entities extracted into the simulation world.
#[derive(Component, Clone)]
struct Simulated;

/// A client request, which arrives at a known time.
#[derive(Component, Clone, Debug)]
struct ClientRequest {
    arrival: SimulationTime,
}

/// The states of a client request.
#[derive(Component, Clone, Debug, PartialEq)]
enum ClientRequestState {
    /// Has not arrived yet.
    Pending,
    /// Has arrived and is waiting for the server.
    Queued,
    /// Has been served.
    Served,
}

/// The server, which serves one client request at a time.
#[derive(Component, Clone, Debug)]
enum ServerState {
    Idle,
    Serving {
        active_request: Entity,
        until: SimulationTime,
    },
}

/// Marks a client request as served and frees its server.
#[derive(Clone, Debug)]
struct CompleteClientReuest {
    server: Entity,
    request: Entity,
}

impl Command for CompleteClientReuest {
    fn apply(self, world: &mut World) {
        world
            .entity_mut(self.request)
            .insert(ClientRequestState::Served);
        world.entity_mut(self.server).insert(ServerState::Idle);
    }
}

/// Predicts each client request joining the queue when it arrives.
fn client_request(
    requests: Query<(Entity, &ClientRequest, &ClientRequestState)>,
    mut states: CandidateComponentEventWriter<ClientRequestState>,
) {
    for (entity, request, state) in &requests {
        if *state == ClientRequestState::Pending {
            states.predict(request.arrival, entity, ClientRequestState::Queued);
        }
    }
}

/// Predicts an idle server starting to serve the next queued client request,
/// and the completion of a service already in progress.
fn server(
    servers: Query<(Entity, &ServerState)>,
    requests: Query<(Entity, &ClientRequestState)>,
    clock: Res<SimulationClock>,
    mut serving: CandidateComponentEventWriter<ServerState>,
    mut events: CandidateEventWriter,
) {
    for (entity, server) in &servers {
        match server {
            ServerState::Idle => {
                let queued = requests
                    .iter()
                    .find(|(_, state)| **state == ClientRequestState::Queued);
                if let Some((request, _)) = queued {
                    let until = clock.now() + SERVER_RESPONSE_TIME;
                    serving.predict_now(
                        entity,
                        ServerState::Serving {
                            active_request: request,
                            until,
                        },
                    );
                }
            }
            ServerState::Serving {
                active_request: request,
                until,
            } => events.predict(
                *until,
                CompleteClientReuest {
                    server: entity,
                    request: *request,
                },
            ),
        }
    }
}

/// Spawns the entities of the model, then builds and spawns the simulation.
fn setup(world: &mut World) {
    world.spawn((Simulated, ServerState::Idle));

    let mut rng = StdRng::seed_from_u64(RANDOMNESS_SEED);
    for _ in 0..NUM_CLIENT_REQUESTS {
        let arrival = rng.gen_range(0.0..=MAX_ARRIVAL_TIME);
        world.spawn((
            Simulated,
            ClientRequest {
                arrival: SimulationTime::new(Duration::from_secs_f32(arrival)),
            },
            ClientRequestState::Pending,
        ));
    }

    let simulation = SimulationBuilder::<Simulated>::new()
        // State extracted into the simulation world.
        .register_component::<ClientRequest>()
        .register_component::<ClientRequestState>()
        .register_component::<ServerState>()
        // The models used to compute the simulation.
        .add_prediction_systems((client_request, server))
        .build(world);

    world.spawn(simulation);
}

/// Prints all results.
fn report(simulations: Query<&Simulation>, mut exit: EventWriter<AppExit>) {
    for simulation in &simulations {
        if simulation.state() == SimulationComputeState::Computing {
            return;
        }
        for (time, step) in simulation.steps() {
            for event in step.events() {
                info!("[{:?}s] {:?}", time.elapsed().as_secs(), event);
            }
        }
    }
    exit.write(AppExit::Success);
}

fn main() {
    App::new()
        .add_plugins((
            MinimalPlugins,
            bevy::log::LogPlugin::default(),
            SimulationPlugin,
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, report)
        .run();
}
