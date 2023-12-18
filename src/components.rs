use std::hash::BuildHasher;
use std::hash::{Hash, Hasher};
use bevy::prelude::*;
use bevy::utils::FixedState;

#[derive(Component, Clone, Copy)]
pub struct Player {
    pub handle: usize,
}

#[derive(Component, Clone, Copy)]
pub struct BulletReady(pub bool);

#[derive(Component)]
pub struct Bullet;

#[derive(Component, Clone, Copy)]
pub struct MoveDir(pub Vec2);

pub fn checksum_transform(transform: &Transform) -> u64 {
    let mut hasher = FixedState.build_hasher();

    assert!(
        transform.translation.is_finite() && transform.rotation.is_finite(),
        "Hashing is not stable for NaN f32 values."
    );

    transform.translation.x.to_bits().hash(&mut hasher);
    transform.translation.y.to_bits().hash(&mut hasher);
    transform.translation.z.to_bits().hash(&mut hasher);

    transform.rotation.x.to_bits().hash(&mut hasher);
    transform.rotation.y.to_bits().hash(&mut hasher);
    transform.rotation.z.to_bits().hash(&mut hasher);
    transform.rotation.w.to_bits().hash(&mut hasher);

    hasher.finish()
}
