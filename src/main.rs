mod args;
mod components;
mod input;

use args::Args;
use crate::ggrs::DesyncDetection;
use bevy::{prelude::*, render::camera::ScalingMode};
use bevy_asset_loader::prelude::*;
use bevy_ggrs::{*, prelude::GgrsEvent};
use bevy_matchbox::{
    matchbox_socket::PeerId,
    prelude::*,
};
use bevy_roll_safe::prelude::*;
use clap::Parser;
use components::*;
use input::*;

type Config = bevy_ggrs::GgrsConfig<u8, PeerId>;

const MAP_SIZE: u32 = 41;
const GRID_WIDTH: f32 = 0.05;
const PLAYER_RADIUS: f32 = 0.5;
const BULLET_RADIUS: f32 = 0.025;

#[derive(AssetCollection, Resource)]
struct ImageAssets {
    #[asset(path = "bullet.png")]
    bullet: Handle<Image>,
}

#[derive(Resource, Clone, Deref, DerefMut)]
struct RoundEndTimer(Timer);

impl Default for RoundEndTimer {
    fn default() -> Self {
        RoundEndTimer(Timer::from_seconds(1.0, TimerMode::Once))
    }
}

#[derive(States, Clone, Eq, PartialEq, Debug, Hash, Default)]
enum GameState {
    #[default]
    AssetLoading,
    Matchmaking,
    InGame,
}

#[derive(States, Clone, Eq, PartialEq, Debug, Hash, Default, Reflect)]
enum RollbackState {
    #[default]
    // Characters are alive and shooting at each other
    InRound,
    // Transition to next round when a character dies
    RoundEnd
}

fn main() {
    let args = Args::parse();
    eprintln!("{args:?}");

    App::new()
        .insert_resource(args)
        .add_state::<GameState>()
        .add_roll_state::<RollbackState>(GgrsSchedule)
        .add_loading_state(
            LoadingState::new(GameState::AssetLoading).continue_to_state(GameState::Matchmaking),
        )
        .add_collection_to_loading_state::<_, ImageAssets>(GameState::AssetLoading)
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    fit_canvas_to_parent: true,
                    ..default()
                }),
                ..default()
            }),
            GgrsPlugin::<Config>::default(),
        ))
        .add_ggrs_state::<RollbackState>()
        .rollback_resource_with_clone::<RoundEndTimer>()
        .rollback_component_with_clone::<Transform>()
        .rollback_component_with_clone::<Sprite>()
        .rollback_component_with_clone::<GlobalTransform>()
        .rollback_component_with_clone::<Handle<Image>>()
        .rollback_component_with_clone::<Visibility>()
        .rollback_component_with_clone::<InheritedVisibility>()
        .rollback_component_with_clone::<ViewVisibility>()
        .rollback_component_with_copy::<BulletReady>()
        .rollback_component_with_copy::<MoveDir>()
        .rollback_component_with_copy::<Player>()
        .checksum_component::<Transform>(checksum_transform)
        .init_resource::<RoundEndTimer>()
        .insert_resource(ClearColor(Color::rgb(0.53, 0.53, 0.53)))
        .add_systems(
            OnEnter(GameState::Matchmaking),
            (setup, start_matchbox_socket.run_if(p2p_mode)),
        )
        .add_systems(OnEnter(GameState::InGame), spawn_players)
        .add_systems(
            Update,
            (
                (
                    wait_for_players.run_if(p2p_mode),
                    start_synctest_session.run_if(synctest_mode),
                )
                    .run_if(in_state(GameState::Matchmaking)),
                (
                    camera_follow,
                    handle_ggrs_events
                ).run_if(in_state(GameState::InGame)),
            ),
        )
        .add_systems(ReadInputs, read_local_inputs)
        .add_systems(
            GgrsSchedule,
            (
                move_players,
                reload_bullet,
                fire_bullets.after(move_players).after(reload_bullet),
                move_bullet.after(fire_bullets),
                kill_players.after(move_bullet).after(move_players),
            )
                .after(apply_state_transition::<RollbackState>)
                .run_if(in_state(RollbackState::InRound)),
        )
        .add_systems(
            GgrsSchedule,
            round_end_timeout
                .ambiguous_with(kill_players)
                .run_if(in_state(RollbackState::RoundEnd))
                .after(apply_state_transition::<RollbackState>),
        )
        .run();
}

fn setup(mut commands: Commands) {
    let mut camera_bundle = Camera2dBundle::default();
    camera_bundle.projection.scaling_mode = ScalingMode::FixedVertical(10.);
    commands.spawn(camera_bundle);

    for i in 0..=MAP_SIZE {
        commands.spawn(SpriteBundle {
            transform: Transform::from_translation(Vec3::new(
                0.,
                i as f32 - MAP_SIZE as f32 / 2.,
                0.,
            )),
            sprite: Sprite {
                color: Color::rgb(0.27, 0.27, 0.27),
                custom_size: Some(Vec2::new(MAP_SIZE as f32, GRID_WIDTH)),
                ..default()
            },
            ..default()
        });
    }

    for i in 0..=MAP_SIZE {
        commands.spawn(SpriteBundle {
            transform: Transform::from_translation(Vec3::new(
                i as f32 - MAP_SIZE as f32 / 2.,
                0.,
                0.,
            )),
            sprite: Sprite {
                color: Color::rgb(0.27, 0.27, 0.27),
                custom_size: Some(Vec2::new(GRID_WIDTH, MAP_SIZE as f32)),
                ..default()
            },
            ..default()
        });
    }
}

fn spawn_players(mut commands: Commands) {
    commands
        .spawn((
            Player { handle: 0 },
            BulletReady(true),
            MoveDir(-Vec2::X),
            SpriteBundle {
                transform: Transform::from_translation(Vec3::new(-2., 0., 100.)),
                sprite: Sprite {
                    color: Color::rgb(0., 0.47, 1.),
                    custom_size: Some(Vec2::new(1., 1.)),
                    ..default()
                },
                ..default()
            },
        ))
        .add_rollback();

    commands
        .spawn((
            Player { handle: 1 },
            BulletReady(true),
            MoveDir(Vec2::X),
            SpriteBundle {
                transform: Transform::from_translation(Vec3::new(2., 0., 100.)),
                sprite: Sprite {
                    color: Color::rgb(0., 0.4, 0.),
                    custom_size: Some(Vec2::new(1., 1.)),
                    ..default()
                },
                ..default()
            },
        ))
        .add_rollback();
}

fn move_players(
    inputs: Res<PlayerInputs<Config>>,
    mut players: Query<(&mut Transform, &mut MoveDir, &Player)>,
    time: Res<Time>,
) {
    for (mut transform, mut move_dir, player) in &mut players {
        let (input, _) = inputs[player.handle];

        let direction = calculate_direction(input);

        if direction == Vec2::ZERO {
            continue;
        }

        move_dir.0 = direction;

        let move_speed = 7.;
        let move_delta = direction * move_speed * time.delta_seconds();

        let old_pos = transform.translation.xy();
        let limit = Vec2::splat(MAP_SIZE as f32 / 2. - 0.5);
        let new_pos = (old_pos + move_delta).clamp(-limit, limit);

        transform.translation.x = new_pos.x;
        transform.translation.y = new_pos.y;
    }
}

fn start_matchbox_socket(mut commands: Commands) {
    let room_url = "ws://127.0.0.1:3536/extreme_bevy?next=2";
    info!("connecting to matchbox server: {room_url}");
    commands.insert_resource(MatchboxSocket::new_ggrs(room_url));
}

fn wait_for_players(
    mut commands: Commands,
    mut socket: ResMut<MatchboxSocket<SingleChannel>>,
    mut next_state: ResMut<NextState<GameState>>,
    args: Res<Args>,
) {
    if socket.get_channel(0).is_err() {
        return;
    }

    socket.update_peers();
    let players = socket.players();

    let num_players = 2;
    if players.len() < num_players {
        return;
    }

    info!("All peers have joined going in-game");

    let mut session_builder = ggrs::SessionBuilder::<Config>::new()
        .with_num_players(num_players)
        .with_desync_detection_mode(DesyncDetection::On { interval: 1})
        .with_input_delay(args.input_delay);

    for (i, player) in players.into_iter().enumerate() {
        session_builder = session_builder
            .add_player(player, i)
            .expect("failed to add player");
    }

    let channel = socket.take_channel(0).unwrap();

    let ggrs_session = session_builder
        .start_p2p_session(channel)
        .expect("failed to start session");

    commands.insert_resource(bevy_ggrs::Session::P2P(ggrs_session));

    next_state.set(GameState::InGame);
}

fn camera_follow(
    local_players: Res<LocalPlayers>,
    players: Query<(&Player, &Transform)>,
    mut cameras: Query<&mut Transform, (With<Camera>, Without<Player>)>,
) {
    for (player, player_transform) in &players {
        if !local_players.0.contains(&player.handle) {
            continue;
        }

        let pos = player_transform.translation;

        for mut transform in &mut cameras {
            transform.translation.x = pos.x;
            transform.translation.y = pos.y;
        }
    }
}

fn fire_bullets(
    mut commands: Commands,
    inputs: Res<PlayerInputs<Config>>,
    images: Res<ImageAssets>,
    mut players: Query<(&Transform, &Player, &mut BulletReady, &MoveDir)>,
) {
    for (transform, player, mut bullet_ready, move_dir) in &mut players {
        let (input, _) = inputs[player.handle];

        if fire(input) && bullet_ready.0 {
            let player_pos = transform.translation.xy();
            let pos = player_pos + move_dir.0 * PLAYER_RADIUS + BULLET_RADIUS;

            commands
                .spawn((
                    Bullet,
                    *move_dir,
                    SpriteBundle {
                        transform: Transform::from_translation(pos.extend(200.))
                            .with_rotation(Quat::from_rotation_arc_2d(Vec2::X, move_dir.0)),
                        texture: images.bullet.clone(),
                        sprite: Sprite {
                            custom_size: Some(Vec2::new(0.3, 0.1)),
                            ..default()
                        },
                        ..default()
                    },
                ))
                .add_rollback();

            bullet_ready.0 = false;
        }
    }
}

fn reload_bullet(
    inputs: Res<PlayerInputs<Config>>,
    mut players: Query<(&mut BulletReady, &Player)>,
) {
    for (mut can_fire, player) in players.iter_mut() {
        let (input, _) = inputs[player.handle];

        if !fire(input) {
            can_fire.0 = true;
        }
    }
}

fn move_bullet(mut bullets: Query<(&mut Transform, &MoveDir), With<Bullet>>, time: Res<Time>) {
    for (mut transform, dir) in &mut bullets {
        let speed = 20.;
        let delta = dir.0 * speed * time.delta_seconds();
        transform.translation += delta.extend(0.);
    }
}

fn kill_players(
    mut commands: Commands,
    players: Query<(Entity, &Transform), (With<Player>, Without<Bullet>)>,
    bullets: Query<&Transform, With<Bullet>>,
    mut next_state: ResMut<NextState<RollbackState>>,
) {
    for (player, player_transform) in &players {
        for bullet_transform in &bullets {
            let distance = Vec2::distance(
                player_transform.translation.xy(),
                bullet_transform.translation.xy(),
            );
            if distance < PLAYER_RADIUS + BULLET_RADIUS {
                commands.entity(player).despawn_recursive();
                next_state.set(RollbackState::RoundEnd);
            }
        }
    }
}

fn synctest_mode(args: Res<Args>) -> bool {
    args.synctest
}

fn p2p_mode(args: Res<Args>) -> bool {
    !args.synctest
}

fn start_synctest_session(mut commands: Commands, mut next_state: ResMut<NextState<GameState>>) {
    info!("Starting synctest session");
    let num_players = 2;

    let mut session_builder = ggrs::SessionBuilder::<Config>::new().with_num_players(num_players);

    for i in 0..num_players {
        session_builder = session_builder
            .add_player(ggrs::PlayerType::Local, i)
            .expect("failed to add player");
    }

    let ggrs_session = session_builder
        .start_synctest_session()
        .expect("failed to start session");

    commands.insert_resource(bevy_ggrs::Session::SyncTest(ggrs_session));
    next_state.set(GameState::InGame);
}

fn handle_ggrs_events(mut session: ResMut<Session<Config>>) {
    match session.as_mut() {
        Session::P2P(s) => {
            for event in s.events() {
                match event {
                    GgrsEvent::Disconnected { .. } | GgrsEvent::NetworkInterrupted { .. } => {
                        warn!("GGRS event: {event:?}")
                    }
                    GgrsEvent::DesyncDetected {
                        local_checksum,
                        remote_checksum,
                        frame,
                        ..
                    } => {
                        error!("Desync on frame {frame}. Local checksum: {local_checksum:X}, remote checksum: {remote_checksum:X}");
                    }
                    _ => info!("GGRS event: {event:?}"),
                }
            }
        }
        _ => {}
    }
}

fn round_end_timeout(
    mut timer: ResMut<RoundEndTimer>,
    mut state: ResMut<NextState<RollbackState>>,
    time: Res<Time>,
) {
    timer.tick(time.delta());

    if timer.just_finished() {
        state.set(RollbackState::InRound);
    }
}
