//! We represent all objects that handle products, including collection and
//! deposit areas, AMRs, processing stations, and conveyors between each processing station
//! as a [`Processor`], that processes objects with a time sampled from a normal distribution
//! specified in [`ProcessorParams`];

use crate::*;
use rand::SeedableRng;
use rand::distributions::Distribution;
use rand::rngs::StdRng;
use rand_distr::Normal;
use rmf_site_sim::compute::SimulationComputeSettings;
use rmf_site_sim::event::CandidateEventWriter;
use rmf_site_sim::{Simulation, SimulationBuilder};

const RANDOMNESS_SEED: u64 = 12345;

// Simulation size
pub const PRODUCT_COUNT: usize = 200;
pub const STATION_COUNT: usize = 4;

// Simulation params
const MAX_EVENTS_PER_STEP: u64 = 256;
const PLAYBACK_SPEED: f32 = 50.0;
const PAUSE_BEFORE_REPLAY: Duration = Duration::from_secs(1);

// Process times, as the mean and standard deviation of a normal distribution.
const AREA_PROCESS_TIME: ProcessorParams = ProcessorParams::new(0.0, 0.0);
const STATION_PROCESS_TIME: ProcessorParams = ProcessorParams::new(6.0, 2.5);
const CONVEYOR_PROCESS_TIME: ProcessorParams = ProcessorParams::new(12.0, 0.0);
const AUTONOMOUS_MOBILE_ROBOT_TRAVEL_TIME: ProcessorParams = ProcessorParams::new(13.0, 6.0);

/// The number of products an autonomous mobile robot carries in one load.
const AUTONOMOUS_MOBILE_ROBOT_CAPACITY: usize = 5;

// Positioning
/// The distance between stations.
const STATION_SPACING: f32 = 190.0;
/// The distance between an area and the station at that end of the line.
pub const AREA_STATION_DISTANCE: f32 = 130.0;

/// A marker component for entities extracted into the simulation world.
#[derive(Component, Clone)]
pub struct Simulated;

/// A 1-dimensional position.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct Position(pub f32);

impl Position {
    pub fn offset(self, distance: f32) -> Position {
        Position(self.0 + distance)
    }

    pub fn midpoint(self, other: Position) -> Position {
        Position((self.0 + other.0) / 2.0)
    }

    pub fn distance_to(self, other: Position) -> f32 {
        (other.0 - self.0).abs()
    }
}

/// A translation between positions.
#[derive(Component, Clone, Copy, Debug)]
pub struct Motion {
    from: Position,
    to: Position,
    start: SimulationTime,
    arrival: SimulationTime,
}

impl Motion {
    /// The interpolated position at a time.
    pub fn at(&self, time: SimulationTime) -> Position {
        let duration = self.arrival.elapsed().saturating_sub(self.start.elapsed());
        if duration.is_zero() {
            return self.to;
        }
        let progress = (time.elapsed().saturating_sub(self.start.elapsed())).as_secs_f32()
            / duration.as_secs_f32();
        Position(self.from.0 + (self.to.0 - self.from.0) * progress.clamp(0.0, 1.0))
    }
}

/// A product with a unique identifier.
#[derive(Component, Clone, Copy, Debug)]
pub struct Product(pub usize);

/// The current [`ProductProcessor`] entity holding the product.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct HeldBy(pub Entity);

/// An entity that can store, process, and pass products.
#[derive(Component, Clone, Copy, Debug)]
pub struct Processor {
    pub params: ProcessorParams,
    pub capacity: ProcessorCapacity,
    pub next: Option<Entity>,
}

/// The parameters of a processing step as the normal distribution.
#[derive(Clone, Copy, Debug)]
pub struct ProcessorParams {
    pub mean: f32,
    pub std_dev: f32,
}

impl ProcessorParams {
    pub const fn new(mean: f32, std_dev: f32) -> Self {
        Self { mean, std_dev }
    }

    /// A duration drawn from this distribution.
    fn sample(&self, rng: &mut StdRng) -> Duration {
        let sampled = Normal::new(self.mean, self.std_dev)
            .expect("A process time distribution should be valid")
            .sample(rng);
        Duration::from_secs_f32(sampled.max(0.0))
    }
}

/// A processor's work capacity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProcessorCapacity {
    /// A maximum number of products that can be processed concurrently.
    Flow(usize),
    /// Only an exact number of products are processed at once.
    Batch(usize),
}

impl ProcessorCapacity {
    pub fn size(&self) -> usize {
        match *self {
            ProcessorCapacity::Flow(size) | ProcessorCapacity::Batch(size) => size,
        }
    }

    pub fn is_batch(&self) -> bool {
        matches!(self, ProcessorCapacity::Batch(_))
    }
}

/// The processors an AMR loads from and delivers to.
#[derive(Component, Clone, Copy, Debug)]
pub struct TransportAssignment {
    pub pickup: Entity,
    pub dropoff: Entity,
}

/// Pass a product to a subsequent processor.
#[derive(Clone, Debug)]
pub struct PassProduct {
    product: Entity,
    to: Entity,
    motion: Option<Motion>,
}

impl Command for PassProduct {
    fn apply(self, world: &mut World) {
        let mut product = world.entity_mut(self.product);
        product.insert(HeldBy(self.to));
        if let Some(work) = self.motion {
            product.insert((work.from, work));
        }
    }
}

/// Mark an entity's departure from its current position and motion towards the target.
#[derive(Clone, Debug)]
pub struct MoveTo {
    entity: Entity,
    motion: Motion,
}

impl Command for MoveTo {
    fn apply(self, world: &mut World) {
        world
            .entity_mut(self.entity)
            .insert((self.motion.from, self.motion));
    }
}

/// Mark an entity's arrival upon completing a motion.
#[derive(Clone, Debug)]
pub struct ArriveAt {
    entity: Entity,
    position: Position,
}

impl Command for ArriveAt {
    fn apply(self, world: &mut World) {
        world
            .entity_mut(self.entity)
            .insert(self.position)
            .remove::<Motion>();
    }
}

pub fn setup(world: &mut World) {
    world.spawn(Camera2d);

    let line = spawn_complete_assembly_line(world);
    let source = *line
        .first()
        .and_then(|area| world.get::<Position>(*area))
        .expect("The line should begin with an area");

    for index in 0..PRODUCT_COUNT {
        let product = world
            .spawn((
                Simulated,
                Name::new(format!("product_{index}")),
                Product(index),
                HeldBy(line[0]),
                source,
            ))
            .id();
        // TODO
        spawn_label(
            world,
            product,
            row_height(PRODUCT_ROW_IDX),
            Anchor::CenterLeft,
        );
    }

    let simulation = build_simulation(world);
    let simulation_entity = world.spawn(simulation).id();

    world.send_event(SimulationPlaybackCommand::SetActiveSimulation(Some(
        simulation_entity,
    )));
    world.send_event(SimulationPlaybackCommand::SetReplayBehaviour(
        SimulationReplayBehaviour(Some(PAUSE_BEFORE_REPLAY)),
    ));
    world.send_event(SimulationPlaybackCommand::SetSpeed(PLAYBACK_SPEED));
    world.send_event(SimulationPlaybackCommand::Play);
}

fn spawn_complete_assembly_line(world: &mut World) -> Vec<Entity> {
    let source = station_position(0).offset(-AREA_STATION_DISTANCE);
    let sink = station_position(STATION_COUNT - 1).offset(AREA_STATION_DISTANCE);

    let mut line = vec![
        spawn_area(world, "area_1", source),
        spawn_autonomous_mobile_robot(world, "amr_1", source),
    ];
    for index in 0..STATION_COUNT {
        if index > 0 {
            line.push(spawn_conveyor(world, index));
        }
        line.push(spawn_station(world, index));
    }
    line.push(spawn_autonomous_mobile_robot(
        world,
        "amr_2",
        station_position(STATION_COUNT - 1),
    ));
    line.push(spawn_area(world, "area_2", sink));

    for (index, processor) in line.iter().enumerate() {
        world
            .entity_mut(*processor)
            .insert(DrawColor(participant_color(index)));

        let previous = index.checked_sub(1).and_then(|index| line.get(index));
        let next = line.get(index + 1);
        let is_batch = world
            .get::<Processor>(*processor)
            .is_some_and(|spec| spec.capacity.is_batch());

        if is_batch {
            if let (Some(pickup), Some(dropoff)) = (previous, next) {
                world.entity_mut(*processor).insert(TransportAssignment {
                    pickup: *pickup,
                    dropoff: *dropoff,
                });
            }
        } else if let Some(next) = next
            && let Some(mut spec) = world.get_mut::<Processor>(*processor)
        {
            spec.next = Some(*next);
        }
    }

    line
}

fn spawn_area(world: &mut World, name: &str, position: Position) -> Entity {
    spawn_processor(
        world,
        name.to_string(),
        position,
        AREA_PROCESS_TIME,
        ProcessorCapacity::Flow(PRODUCT_COUNT),
        PROCESSOR_ROW_IDX,
        Vec2::splat(AREA_SIZE),
    )
}

fn spawn_station(world: &mut World, index: usize) -> Entity {
    spawn_processor(
        world,
        format!("station_{}", index + 1),
        station_position(index),
        STATION_PROCESS_TIME,
        ProcessorCapacity::Flow(1),
        PROCESSOR_ROW_IDX,
        Vec2::splat(STATION_SIZE),
    )
}

fn spawn_conveyor(world: &mut World, index: usize) -> Entity {
    let (input, output) = (station_position(index - 1), station_position(index));
    spawn_processor(
        world,
        format!("conveyor_{index}"),
        input.midpoint(output),
        CONVEYOR_PROCESS_TIME,
        ProcessorCapacity::Flow(1),
        PROCESSOR_ROW_IDX,
        Vec2::new(input.distance_to(output), CONVEYOR_SIZE),
    )
}

fn spawn_autonomous_mobile_robot(world: &mut World, name: &str, position: Position) -> Entity {
    spawn_processor(
        world,
        name.to_string(),
        position,
        AUTONOMOUS_MOBILE_ROBOT_TRAVEL_TIME,
        ProcessorCapacity::Batch(AUTONOMOUS_MOBILE_ROBOT_CAPACITY),
        ROBOT_ROW_IDX,
        Vec2::splat(AUTONOMOUS_MOBILE_ROBOT_SIZE),
    )
}

fn spawn_processor(
    world: &mut World,
    name: String,
    position: Position,
    process_params: ProcessorParams,
    capacity: ProcessorCapacity,
    row: usize,
    size: Vec2,
) -> Entity {
    let processor = world
        .spawn((
            Simulated,
            Name::new(name),
            Processor {
                params: process_params,
                capacity,
                next: None,
            },
            position,
            Row(row),
            DrawSize(size),
        ))
        .id();
    spawn_label(
        world,
        processor,
        row_height(row) - size.y / 2.0 - LABEL_GAP,
        Anchor::Center,
    );
    processor
}

pub fn station_position(index: usize) -> Position {
    let first = -(STATION_COUNT as f32 - 1.0) * STATION_SPACING / 2.0;
    Position(first + index as f32 * STATION_SPACING)
}

fn build_simulation(world: &World) -> Simulation {
    SimulationBuilder::<Simulated>::new()
        .set_compute_settings(SimulationComputeSettings {
            max_events_per_step: Some(MAX_EVENTS_PER_STEP),
        })
        .register_component::<Name>()
        .register_component::<Position>()
        .register_component::<Product>()
        .register_component::<HeldBy>()
        .register_component::<Processor>()
        .register_component::<TransportAssignment>()
        .add_plugins(|app: &mut App| {
            app.insert_resource(SimulationRng(StdRng::seed_from_u64(RANDOMNESS_SEED)));
        })
        .add_prediction_systems((motion, processor, autonomous_mobile_robot).chain())
        .add_visualization_systems((animate_motion, (draw_processors, draw_products)).chain())
        .build(world)
}

#[derive(Resource)]
struct SimulationRng(StdRng);

fn motion(moving: Query<(Entity, &Motion)>, mut events: CandidateEventWriter) {
    for (entity, motion) in &moving {
        events.predict(
            motion.arrival,
            ArriveAt {
                entity,
                position: motion.to,
            },
        );
    }
}

fn processor(
    processors: Query<(
        Entity,
        &Processor,
        &Position,
        &Name,
        Option<&TransportAssignment>,
    )>,
    products: ProductQuery,
    mut rng: ResMut<SimulationRng>,
    clock: Res<SimulationClock>,
    mut events: CandidateEventWriter,
) {
    let now = clock.now();

    for (entity, processor, position, name, assignment) in &processors {
        let Some(next) = assignment
            .map(|assignment| assignment.dropoff)
            .or(processor.next)
        else {
            continue;
        };
        let Ok((_, receiver, receiver_position, _, _)) = processors.get(next) else {
            continue;
        };
        let Some((product, index)) = ready_product(&products, entity) else {
            continue;
        };
        if held(&products, next) >= receiver.capacity.size() {
            continue;
        }
        if (processor.capacity.is_batch() || receiver.capacity.is_batch())
            && position != receiver_position
        {
            continue;
        }

        let work = (!receiver.capacity.is_batch()).then(|| {
            let duration = receiver.params.sample(&mut rng.0);
            motion_to(*position, *receiver_position, duration, now)
        });
        events.predict_now(PassProduct {
            product,
            to: next,
            motion: work,
        });
        info!("[processor] {} passed product {} on", name, index);
    }
}

/// Drives each AMR between the processor it loads from and the one it feeds.
fn autonomous_mobile_robot(
    autonomous_mobile_robots: Query<
        (Entity, &Processor, &TransportAssignment, &Position, &Name),
        Without<Motion>,
    >,
    positions: Query<&Position>,
    products: ProductQuery,
    mut rng: ResMut<SimulationRng>,
    clock: Res<SimulationClock>,
    mut events: CandidateEventWriter,
) {
    let now = clock.now();

    for (entity, processor, assignment, position, name) in &autonomous_mobile_robots {
        let ProcessorCapacity::Batch(size) = processor.capacity else {
            continue;
        };
        let (Ok(pickup_position), Ok(dropoff_position)) = (
            positions.get(assignment.pickup),
            positions.get(assignment.dropoff),
        ) else {
            continue;
        };

        let load = held(&products, entity);
        let waiting = held(&products, assignment.pickup);
        let target = if load == size || (load > 0 && waiting == 0) {
            dropoff_position
        } else if load == 0 && waiting > 0 {
            pickup_position
        } else {
            continue;
        };
        if position == target {
            continue;
        }

        let duration = processor.params.sample(&mut rng.0);
        let motion = motion_to(*position, *target, duration, now);
        events.predict_now(MoveTo { entity, motion });
        info!(
            "[autonomous_mobile_robot] {} is driving, arriving at {:?}",
            name, motion.arrival
        );
    }
}

pub type ProductQuery<'w, 's> =
    Query<'w, 's, (Entity, &'static Product, &'static HeldBy, Has<Motion>)>;

fn ready_product(products: &ProductQuery, processor: Entity) -> Option<(Entity, usize)> {
    products
        .iter()
        .filter(|(_, _, held_by, working)| held_by.0 == processor && !working)
        .min_by_key(|(_, product, _, _)| product.0)
        .map(|(entity, product, _, _)| (entity, product.0))
}

fn held(products: &ProductQuery, processor: Entity) -> usize {
    products
        .iter()
        .filter(|(_, _, held_by, _)| held_by.0 == processor)
        .count()
}

fn motion_to(from: Position, to: Position, duration: Duration, now: SimulationTime) -> Motion {
    Motion {
        from,
        to,
        start: now,
        arrival: now + duration,
    }
}
