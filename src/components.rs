use bevy::prelude::*;

#[derive(Component)]
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
    //todo: produce u64 based on value of transform
}
