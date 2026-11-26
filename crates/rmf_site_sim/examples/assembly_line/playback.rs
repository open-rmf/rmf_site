use crate::*;
use bevy::color::{Color, palettes::basic};
use bevy::sprite::Anchor;

// Rows
// Each entity will be used to visualize a single entity type during visualization.
// ```text
// row 0 : products
// row 1 : areas, stations, conveyors
// row 2 : robots
pub const PRODUCT_ROW_IDX: usize = 0;
pub const PROCESSOR_ROW_IDX: usize = 1;
pub const ROBOT_ROW_IDX: usize = 2;
const ROW_TOP: f32 = 150.0;
const ROW_SPACING: f32 = 90.0;
// Sizes
const PRODUCT_SIZE: f32 = 14.0;
pub const AREA_SIZE: f32 = 90.0;
pub const AUTONOMOUS_MOBILE_ROBOT_SIZE: f32 = 40.0;
pub const STATION_SIZE: f32 = 70.0;
pub const CONVEYOR_SIZE: f32 = 16.0;
//
const BUSY_INDICATOR_OFFSET: f32 = 8.0;
const PRODUCT_STACK_SPACING: f32 = 18.0;
const LABEL_FONT_SIZE: f32 = 11.0;
pub const LABEL_GAP: f32 = 12.0;
const PRODUCT_LABEL_GAP: f32 = 6.0;

/// The row a category of participant is drawn in.
#[derive(Component, Clone, Copy, Debug)]
pub struct Row(pub usize);

/// The size a processor is drawn at.
#[derive(Component, Clone, Copy, Debug)]
pub struct DrawSize(pub Vec2);

/// An identifying colour.
#[derive(Component, Clone, Copy, Debug)]
pub struct DrawColor(pub Color);

#[derive(Component, Clone, Copy, Debug)]
pub struct Label(pub Entity);

pub fn row_height(row: usize) -> f32 {
    ROW_TOP - row as f32 * ROW_SPACING
}

pub fn participant_color(index: usize) -> Color {
    [
        basic::RED,
        basic::YELLOW,
        basic::LIME,
        basic::AQUA,
        basic::BLUE,
        basic::FUCHSIA,
        basic::MAROON,
        basic::OLIVE,
    ][index % 8]
        .into()
}

pub fn spawn_label(world: &mut World, participant: Entity, height: f32, anchor: Anchor) {
    let name = world
        .get::<Name>(participant)
        .map(Name::to_string)
        .unwrap_or_default();
    let position = world
        .get::<Position>(participant)
        .map(|position| position.0)
        .unwrap_or_default();

    world.spawn((
        Text2d::new(name),
        TextFont {
            font_size: LABEL_FONT_SIZE,
            ..default()
        },
        TextColor(Color::from(basic::SILVER)),
        anchor,
        Transform::from_xyz(position, height, 0.0),
        Label(participant),
    ));
}

pub fn animate_motion(clock: Res<SimulationClock>, mut moving: Query<(&mut Position, &Motion)>) {
    let time = clock.now();

    for (mut position, motion) in &mut moving {
        *position = motion.at(time);
    }
}

pub fn draw_processors(
    processors: Query<(
        Entity,
        &Processor,
        &Position,
        &Row,
        &DrawSize,
        &DrawColor,
        &Name,
        Has<Motion>,
    )>,
    products: Query<(&HeldBy, Has<Motion>)>,
    mut labels: Query<(&Label, &mut Text2d, &mut Transform)>,
    mut gizmos: Gizmos,
) {
    let mut labelled: HashMap<Entity, (String, Vec2)> = HashMap::new();
    for (entity, processor, position, row, size, color, name, driving) in &processors {
        let center = Vec2::new(position.0, row_height(row.0));
        let held: Vec<bool> = products
            .iter()
            .filter(|(held_by, _)| held_by.0 == entity)
            .map(|(_, working)| working)
            .collect();
        let busy = driving || held.iter().any(|working| *working);

        // Only AMRs are visualized as circles.
        if processor.capacity.is_batch() {
            gizmos.circle_2d(center, size.0.x / 2.0, color.0);
            if busy {
                gizmos.circle_2d(center, (size.0.x - BUSY_INDICATOR_OFFSET) / 2.0, color.0);
            }
        } else {
            gizmos.rect_2d(center, size.0, color.0);
            if busy {
                gizmos.rect_2d(center, size.0 - Vec2::splat(BUSY_INDICATOR_OFFSET), color.0);
            }
        }

        let text = format!("{} ({}/{})", name, held.len(), processor.capacity.size());
        labelled.insert(
            entity,
            (text, center - Vec2::Y * (size.0.y / 2.0 + LABEL_GAP)),
        );
    }

    for (label, mut text, mut transform) in &mut labels {
        let Some((labelled, position)) = labelled.get(&label.0) else {
            continue;
        };
        if text.0 != *labelled {
            text.0 = labelled.clone();
        }
        transform.translation.x = position.x;
        transform.translation.y = position.y;
    }
}

pub fn draw_products(
    products: Query<(Entity, &Product, &HeldBy, &Position)>,
    holders: Query<(&Position, &DrawColor, &Processor)>,
    mut labels: Query<(&Label, &mut Transform)>,
    mut gizmos: Gizmos,
) {
    let mut sorted: Vec<_> = products.iter().collect();
    sorted.sort_by_key(|(_, product, _, _)| product.0);

    let mut stacked: HashMap<i32, usize> = HashMap::new();
    let mut label_positions: HashMap<Entity, Vec2> = HashMap::new();
    for (product, _, held_by, position) in sorted {
        let Ok((holder_position, color, holder)) = holders.get(held_by.0) else {
            continue;
        };
        let x = match holder.capacity.is_batch() {
            true => holder_position.0,
            false => position.0,
        };

        let stack = stacked.entry(x.round() as i32).or_default();
        let height = *stack;
        *stack += 1;

        let center = Vec2::new(
            x,
            row_height(PRODUCT_ROW_IDX) + height as f32 * PRODUCT_STACK_SPACING,
        );
        gizmos.rect_2d(center, Vec2::splat(PRODUCT_SIZE), color.0);
        label_positions.insert(
            product,
            center + Vec2::X * (PRODUCT_SIZE / 2.0 + PRODUCT_LABEL_GAP),
        );
    }

    for (label, mut transform) in &mut labels {
        let Some(position) = label_positions.get(&label.0) else {
            continue;
        };
        transform.translation.x = position.x;
        transform.translation.y = position.y;
    }
}
