//! Inspect original stage assets while gameplay and the PS1 renderer are ported.
use bevy::{
    app::{AppExit, ScheduleRunnerPlugin},
    camera::RenderTarget,
    core_pipeline::tonemapping::{DebandDither, Tonemapping},
    prelude::*,
    render::{
        render_resource::{TextureFormat, TextureUsages},
        view::screenshot::{Screenshot, ScreenshotCaptured, save_to_disk},
    },
    window::ExitCondition,
    winit::WinitPlugin,
};
use lsd_assets::{
    StageLinkSession, StreamedStage,
    formats::{Lbd, Tix, Tmd, Vram},
    is_canonical_reference, load_effect_palette, load_layered_stage, load_link_tables,
    load_math_tables, sha256_hex, stage_map_path,
};
use lsd_bevy::asset_scene::AssetSceneBuilder;
use lsd_core::{
    ChunkPlayer, CollisionBounds, CollisionChunk, CollisionPolicy, LinkedPlayer,
    MapCompletionCommand, MapEffect, PadState, PlayerControls, PsxMathTables, SceneCollision,
    SceneLinkTables, SpawnLocation,
};
use std::path::PathBuf;

#[derive(Resource)]
struct StageAssets {
    map: Lbd,
    vram: Vram,
    stage: u8,
    map_number: u16,
    texture_variant: char,
}

#[derive(Resource)]
struct Capture {
    path: PathBuf,
    target: Option<Handle<Image>>,
    requested: bool,
    remaining_ticks: Option<u32>,
}

#[derive(Resource)]
struct WalkingScene {
    player: ChunkPlayer,
    collision: CollisionChunk,
    tables: PsxMathTables,
    layered_collision: Option<SceneCollision>,
    streamed: Option<StreamedStage>,
    linked: Option<LinkedPlayer>,
    effect: Option<MapEffect>,
    session: Option<StageLinkSession>,
}

type ModelMeshes = Vec<Vec<Vec<(Handle<Mesh>, Handle<StandardMaterial>)>>>;
#[derive(Resource)]
struct StreamVisuals {
    revision: u64,
    rebuild_epoch: u64,
    entities: Vec<Entity>,
    models: std::collections::BTreeMap<u16, ModelMeshes>,
}

#[derive(Resource)]
struct LayeredAssets {
    maps: Vec<Lbd>,
    origin: [i32; 3],
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let root = PathBuf::from(args.next().ok_or(
        "Usage: lsd-stage <extracted-files-root> [stage] [map] [texture A-D] [--inspect]",
    )?);
    let stage: u8 = args.next().unwrap_or_else(|| "0".into()).parse()?;
    let map_number: u16 = args.next().unwrap_or_else(|| "0".into()).parse()?;
    let texture = args
        .next()
        .unwrap_or_else(|| "A".into())
        .to_ascii_uppercase();
    let mut walk = false;
    let mut walk_layers = false;
    let mut walk_stream = false;
    let mut walk_links = false;
    let mut walk_steps = None;
    let mut supplied_wall = None;
    let mut live_capture = false;
    let (inspect, capture) = match args.next() {
        None => (false, None),
        Some(value) if value == "--inspect" => (true, None),
        Some(value) if value == "--walk" => {
            walk = true;
            (false, None)
        }
        Some(value) if value == "--walk-layers" => {
            walk = true;
            walk_layers = true;
            (false, None)
        }
        Some(value) if value == "--walk-stream" => {
            walk = true;
            walk_stream = true;
            (false, None)
        }
        Some(value) if value == "--walk-links" => {
            walk = true;
            walk_stream = true;
            walk_links = true;
            (false, None)
        }
        Some(value)
            if matches!(
                value.as_str(),
                "--link-rebuild-capture" | "--link-rebuild-live-capture"
            ) =>
        {
            live_capture = value == "--link-rebuild-live-capture";
            walk = true;
            walk_stream = true;
            walk_links = true;
            let path = PathBuf::from(
                args.next()
                    .ok_or("--link-rebuild-capture needs a PNG path")?,
            );
            supplied_wall = Some(
                args.next()
                    .ok_or("--link-rebuild-capture needs a wall-trigger index")?
                    .parse::<usize>()?,
            );
            walk_steps = Some(
                args.next()
                    .ok_or("--link-rebuild-capture needs a tick count")?
                    .parse::<u32>()?,
            );
            (false, Some(path))
        }
        Some(value)
            if matches!(
                value.as_str(),
                "--walk-capture"
                    | "--walk-layers-capture"
                    | "--walk-stream-capture"
                    | "--walk-links-capture"
            ) =>
        {
            walk = true;
            walk_layers = value == "--walk-layers-capture";
            walk_links = value == "--walk-links-capture";
            walk_stream = value == "--walk-stream-capture" || walk_links;
            let path = PathBuf::from(args.next().ok_or("--walk-capture needs a new PNG path")?);
            walk_steps = Some(
                args.next()
                    .ok_or("--walk-capture needs a tick count")?
                    .parse::<u32>()?,
            );
            (false, Some(path))
        }
        Some(value) if value == "--capture" => (
            false,
            Some(PathBuf::from(
                args.next().ok_or("--capture needs a PNG path")?,
            )),
        ),
        _ => {
            return Err(
                "Use --inspect, --capture, --walk, --walk-layers, --walk-stream, --walk-links or a walking capture after the texture variant"
                    .into(),
            );
        }
    };
    if stage > 13 || !matches!(texture.as_str(), "A" | "B" | "C" | "D") || args.next().is_some() {
        return Err("Stage must be 0..13 and texture variant A..D".into());
    }
    let executable = std::fs::read(root.join("SLPS_015.56"))?;
    if !is_canonical_reference(&executable) {
        return Err("Extraction has a different reference executable".into());
    }
    let folder = root.join(format!("CDI/STG{stage:02}"));
    let map_bytes = std::fs::read(root.join(stage_map_path(&executable, stage, map_number)?))?;
    let texture_bytes = std::fs::read(folder.join(format!("TEX{texture}.TIX")))?;
    let map = Lbd::parse(&map_bytes)?;
    if map.tmd.objects.iter().any(|object| object.scale != 0) {
        return Err("Nonzero TMD design scale requires original runtime scale review".into());
    }
    let tix = Tix::parse(&texture_bytes)?;
    let mut vram = Vram::default();
    let mut display_stage = stage;
    for bank in &tix.banks {
        for image in &bank.images {
            vram.upload(image)?;
        }
    }
    let mut pages = std::collections::BTreeSet::new();
    for polygon in map.tmd.objects.iter().flat_map(|object| &object.polygons) {
        if let (Some(page), Some(clut)) = (polygon.texture_page, polygon.clut) {
            pages.insert((page, clut));
        }
    }
    for (page, clut) in pages {
        vram.texture_rgba(page, clut)?;
    }
    let tiles = (0..400)
        .map(|cell| {
            map.cell_chain(cell)
                .map(|chain| chain.into_iter().filter(|tile| tile.present()).count())
        })
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .sum::<usize>();
    let polygons: usize = map
        .tmd
        .objects
        .iter()
        .map(|object| object.polygons.len())
        .sum();
    println!(
        "stage={stage} map={map_number} texture={texture} objects={} polygons={polygons} tile_instances={tiles}\nmap_sha256={}\ntexture_sha256={}\nprofile=asset_inspection gameplay_parity_accepted=false",
        map.tmd.objects.len(),
        sha256_hex(&map_bytes),
        sha256_hex(&texture_bytes)
    );
    if inspect {
        return Ok(());
    }
    let mut layered_assets = None;
    let walking = if walk {
        let tables = load_math_tables(&executable)?;
        let collision = map.collision_chunk(CollisionPolicy::for_stage(stage));
        let mut streamed = if walk_stream {
            Some(StreamedStage::load(
                &root,
                &executable,
                stage,
                map_number,
                [0; 3],
            )?)
        } else {
            None
        };
        let mut effect = if walk_links {
            Some(MapEffect::new(load_effect_palette(&executable)?))
        } else {
            None
        };
        let layered = if walk_layers {
            Some(load_layered_stage(
                &root,
                &std::fs::read(root.join("SLPS_015.56"))?,
                stage,
                [0, 0, 0],
            )?)
        } else {
            None
        };
        let body = Tmd::parse(&std::fs::read(root.join("CDI/ETC/DREAME5.TMD"))?)?;
        let bounds = CollisionBounds::from_points(
            &body.objects[0]
                .vertices
                .iter()
                .map(|point| point.xyz)
                .collect::<Vec<_>>(),
        )
        .ok_or("Missing original player bounds")?;
        let mut spawn = None;
        for cell in 0..400 {
            for tile in map.cell_chain(cell)? {
                if tile.present()
                    && let Some(contact) = collision.floor(tile.native_position(cell), &tables)?
                {
                    spawn = Some(contact.position);
                    break;
                }
            }
            if spawn.is_some() {
                break;
            }
        }
        let mut player = ChunkPlayer {
            controls: PlayerControls {
                eye: [0, -512, 0],
                target: [0, -512, 1024],
                ..default()
            },
            pad: PadState::default(),
            position: spawn.ok_or("No floor found for the movement prototype")?,
            bounds,
        };
        if let Some(layered) = &layered {
            for (position, offset) in player.position.iter_mut().zip(layered.origin) {
                *position = position.wrapping_add(offset);
            }
        }
        if let Some(streamed) = &streamed {
            let origin = streamed
                .collision
                .slots
                .slots
                .iter()
                .find(|slot| slot.ready && slot.map_index == map_number as i16)
                .ok_or("Requested map is not resident")?
                .origin;
            for (position, offset) in player.position.iter_mut().zip(origin) {
                *position = position.wrapping_add(offset);
            }
        }
        let mut linked = if walk_links {
            let original = load_link_tables(&executable)?;
            // These are the original map-scene movement defaults; this inspector supplies its own spawn/seed/palette.
            player.controls.move_direction = if stage == 0 { 2 } else { 3 };
            player.controls.previous_move_direction = player.controls.move_direction;
            Some(LinkedPlayer::new(
                stage,
                &player,
                &streamed.as_ref().unwrap().collision,
                SceneLinkTables {
                    catalog: original.catalog,
                    motion: original.motion,
                    auto_move: original.auto_move,
                },
                1,
            )?)
        } else {
            None
        };
        let mut session = walk_links.then(StageLinkSession::default);
        if let Some(index) = supplied_wall {
            let linked = linked.as_mut().unwrap();
            let trigger = *linked.tables.catalog.wall.triggers[usize::from(stage)]
                .get(index)
                .ok_or("Wall-trigger index is outside the original stage table")?;
            linked.keys.contact = SpawnLocation {
                chunk: trigger.chunk,
                tile: trigger.tile.max(0),
                position: [0; 3],
            };
            linked.wall_contact(&mut player, &streamed.as_ref().unwrap().collision, 4)?;
            for command in linked.commands.clone() {
                effect.as_mut().unwrap().consume(command)?;
            }
            println!("profile=supplied_wall_link original_collision_encounter_supplied=true");
        }
        if let Some(steps) = walk_steps.filter(|_| !live_capture) {
            for _ in 0..steps {
                if let Some(streamed) = &mut streamed {
                    streamed_step(
                        &mut player,
                        streamed,
                        &tables,
                        linked.as_mut(),
                        effect.as_mut(),
                        session.as_mut(),
                        if supplied_wall.is_some() { 0 } else { 0x1000 },
                    )?;
                } else if let Some(layered) = &layered {
                    player.step_in_scene(0x1000, &layered.collision, &tables)?;
                } else {
                    player.step(0x1000, &collision, &tables)?;
                }
            }
        }
        if let Some(streamed) = &mut streamed {
            streamed.complete_frame()?;
            if streamed.stage_id() != stage {
                display_stage = streamed.stage_id();
                vram = streamed.load_texture_variant(texture.chars().next().unwrap())?;
            }
        }
        println!(
            "profile={} position={:?} heading={} original_link_npc_callbacks_pending=true",
            if walk_links {
                "linked_streamed_scene_prototype"
            } else if walk_stream {
                "streamed_scene_movement_prototype"
            } else if walk_layers {
                "layered_scene_movement_prototype"
            } else {
                "single_chunk_movement_prototype"
            },
            player.position,
            player.controls.angles[1]
        );
        if let Some(streamed) = &streamed {
            println!(
                "resident_maps={:?} revision={} pending={} original_cd_timing_actor_callbacks_pending=true",
                streamed
                    .collision
                    .slots
                    .slots
                    .iter()
                    .filter(|slot| slot.ready)
                    .map(|slot| slot.map_index)
                    .collect::<Vec<_>>(),
                streamed.revision,
                streamed.protocol.pending
            );
        }
        if let Some(linked) = &linked {
            println!(
                "link_event={} map_state={} destination={} spawn={:?} dream_tick={} rng_state={} effect_controller_active=true original_gpu_audio_day_scheduling_pending=true",
                linked.links.execution.state,
                linked.map.state,
                linked.links.execution.stage,
                linked.links.spawn,
                linked.links.execution.dream_timer,
                linked.rng.state()
            );
        }
        if let Some(session) = &session {
            println!(
                "session_state={} rebuilds={} loaded_stage={} pending_day_event={:?}",
                session.policy.state, session.rebuilds, display_stage, session.pending_day_event
            );
        }
        if let Some(effect) = &effect {
            println!(
                "effect_state={} life={} color_step={} render_attached={} map_attached={}",
                effect.effect.state,
                effect.effect.life,
                effect.effect.color_step,
                effect.render_attached,
                effect.map_attached
            );
        }
        if let Some(layered) = &layered {
            println!(
                "loaded_maps={} slot={:?} original_stream_lifecycle_pending=true",
                layered.maps.len(),
                layered
                    .collision
                    .slots
                    .locate(player.position)?
                    .map(|location| location.slot)
            );
        }
        let layered_collision = layered.map(|layered| {
            layered_assets = Some(LayeredAssets {
                maps: layered.maps,
                origin: layered.origin,
            });
            layered.collision
        });
        Some(WalkingScene {
            player,
            collision,
            tables,
            layered_collision,
            streamed,
            linked,
            effect,
            session,
        })
    } else {
        None
    };
    let mut app = App::new();
    if walk_stream {
        app.insert_resource(StreamVisuals {
            revision: u64::MAX,
            rebuild_epoch: u64::MAX,
            entities: Vec::new(),
            models: Default::default(),
        })
        .add_systems(Update, stream_meshes);
    }
    if let Some(layered) = layered_assets {
        app.insert_resource(layered);
    }
    if let Some(walking) = walking {
        app.insert_resource(walking)
            .insert_resource(Time::<Fixed>::from_hz(15.0))
            .add_systems(FixedUpdate, walking_controls)
            .add_systems(Update, walking_camera);
    }
    app.insert_resource(StageAssets {
        map,
        vram,
        stage: display_stage,
        map_number,
        texture_variant: texture.chars().next().unwrap(),
    });
    let plugins = DefaultPlugins
        .set(ImagePlugin::default_nearest())
        .set(WindowPlugin {
            primary_window: if capture.is_some() {
                None
            } else {
                Some(Window {
                    title: format!(
                        "LSD Bevy - STG{stage:02} M{map_number:03} | WASD, Q/E, Shift, R/F"
                    ),
                    resolution: (960, 720).into(),
                    ..default()
                })
            },
            exit_condition: ExitCondition::DontExit,
            ..default()
        });
    if let Some(path) = capture {
        if path.exists() {
            return Err("Capture output already exists".into());
        }
        app.insert_resource(Capture {
            path,
            target: None,
            requested: false,
            remaining_ticks: if live_capture { walk_steps } else { None },
        })
        .add_plugins(plugins.disable::<WinitPlugin>())
        .add_plugins(ScheduleRunnerPlugin::run_loop(
            std::time::Duration::from_secs_f64(1.0 / 60.0),
        ))
        .add_systems(Update, capture_frame);
    } else {
        app.add_plugins(plugins);
    }
    app.add_systems(Startup, setup)
        .add_systems(
            Update,
            camera_controls.run_if(not(resource_exists::<WalkingScene>)),
        )
        .run();
    Ok(())
}

fn walking_controls(
    keys: Res<ButtonInput<KeyCode>>,
    mut scene: ResMut<WalkingScene>,
    mut source: ResMut<StageAssets>,
    capture: Option<ResMut<Capture>>,
) {
    if capture
        .as_ref()
        .is_some_and(|capture| capture.remaining_ticks.is_none_or(|ticks| ticks == 0))
    {
        return;
    }
    let mut buttons = 0;
    for (key, mask) in [
        (KeyCode::KeyW, 0x1000),
        (KeyCode::KeyS, 0x4000),
        (KeyCode::KeyA, 0x8000),
        (KeyCode::KeyD, 0x2000),
        (KeyCode::KeyQ, 1),
        (KeyCode::KeyE, 2),
        (KeyCode::ShiftLeft, 0x40),
        (KeyCode::KeyR, 0x10),
        (KeyCode::KeyF, 0x80),
        (KeyCode::Space, 0x20),
    ] {
        if keys.pressed(key) {
            buttons |= mask;
        }
    }
    if capture.is_some() {
        buttons = 0;
    }
    let WalkingScene {
        player,
        collision,
        tables,
        layered_collision,
        streamed,
        linked,
        effect,
        session,
    } = &mut *scene;
    if let Some(streamed) = streamed {
        streamed_step(
            player,
            streamed,
            tables,
            linked.as_mut(),
            effect.as_mut(),
            session.as_mut(),
            buttons,
        )
        .expect("decoded streamed movement query");
        if source.stage != streamed.stage_id() {
            source.stage = streamed.stage_id();
            source.vram = streamed
                .load_texture_variant(source.texture_variant)
                .expect("validated original destination texture");
        }
    } else if let Some(layered) = layered_collision {
        player
            .step_in_scene(buttons, layered, tables)
            .expect("decoded scene movement query");
    } else {
        player
            .step(buttons, collision, tables)
            .expect("decoded chunk movement query");
    }
    if let Some(mut capture) = capture
        && let Some(ticks) = &mut capture.remaining_ticks
    {
        *ticks -= 1;
        if *ticks == 0
            && let (Some(session), Some(linked), Some(effect), Some(streamed)) = (
                session.as_ref(),
                linked.as_ref(),
                effect.as_ref(),
                streamed.as_ref(),
            )
        {
            println!(
                "live_session_state={} rebuilds={} loaded_stage={} map_state={} effect_state={} rng_state={} dream_tick={}",
                session.policy.state,
                session.rebuilds,
                streamed.stage_id(),
                linked.map.state,
                effect.effect.state,
                linked.rng.state(),
                linked.links.execution.dream_timer
            );
        }
    }
}

fn streamed_step(
    player: &mut ChunkPlayer,
    stage: &mut StreamedStage,
    tables: &PsxMathTables,
    linked: Option<&mut LinkedPlayer>,
    effect: Option<&mut MapEffect>,
    session: Option<&mut StageLinkSession>,
    buttons: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    stage.complete_frame()?;
    if let Some(linked) = linked {
        let mut effect = effect;
        let mut session = session;
        if let (Some(session), Some(effect)) = (session.as_deref_mut(), effect.as_deref_mut()) {
            session.tick(stage, player, linked, effect)?;
            for event in &session.events {
                println!("session_command={event:?}");
            }
        }
        if let Some(effect) = effect.as_deref_mut() {
            for command in effect.tick(&mut linked.map, &mut player.controls)? {
                println!("effect_completion_command={command:?}");
                if let (MapCompletionCommand::Notify(code), Some(session)) =
                    (command, session.as_deref_mut())
                {
                    session.receive(code);
                }
            }
        }
        if session
            .as_deref()
            .is_some_and(|session| session.pending_day_event.is_some())
        {
            player.controls.input_locked = true;
            return Ok(());
        }
        let ready = std::array::from_fn(|i| stage.protocol.slots[i].ready);
        linked.step(player, &stage.collision, tables, ready, buttons)?;
        for command in &linked.commands {
            println!("link_command={command:?}");
            if let Some(effect) = effect.as_deref_mut() {
                for signal in effect.consume(*command)? {
                    println!("map_parent_command={signal:?}");
                    if let (MapCompletionCommand::Notify(code), Some(session)) =
                        (signal, session.as_deref_mut())
                    {
                        session.receive(code);
                    }
                }
            }
        }
        for command in &linked.sounds {
            println!("link_sound_command={command:?}");
        }
        stage.update_for_position(player.position)?;
        linked.refresh_key(&stage.collision, player.position)?;
    } else {
        player.step_in_scene(buttons, &stage.collision, tables)?;
        stage.update_for_position(player.position)?;
    }
    Ok(())
}

fn stream_meshes(
    mut commands: Commands,
    source: Res<StageAssets>,
    scene: Res<WalkingScene>,
    mut visuals: ResMut<StreamVisuals>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let Some(stage) = &scene.streamed else {
        return;
    };
    let epoch = scene.session.as_ref().map_or(0, |session| session.rebuilds);
    if visuals.revision == stage.revision && visuals.rebuild_epoch == epoch {
        return;
    }
    for entity in visuals.entities.drain(..) {
        commands.entity(entity).despawn();
    }
    if visuals.rebuild_epoch != epoch {
        visuals.models.clear();
    }
    let resident: std::collections::BTreeSet<_> = stage
        .collision
        .slots
        .slots
        .iter()
        .filter(|slot| slot.ready)
        .map(|slot| slot.map_index as u16)
        .collect();
    visuals.models.retain(|map, _| resident.contains(map));
    let mut builder =
        AssetSceneBuilder::new(&source.vram, &mut meshes, &mut materials, &mut images);
    for (physical, map) in stage.maps.iter().enumerate() {
        let Some(map) = map else {
            continue;
        };
        let slot = stage.collision.slots.slots[physical];
        let map_number = slot.map_index as u16;
        let models = visuals
            .models
            .entry(map_number)
            .or_insert_with(|| {
                map.tmd
                    .objects
                    .iter()
                    .map(|object| {
                        (0..4)
                            .map(|quarter| builder.build_object(object, quarter))
                            .collect()
                    })
                    .collect()
            })
            .clone();
        for cell in 0..400 {
            for tile in map.cell_chain(cell).expect("validated cell chain") {
                if !tile.present() {
                    continue;
                }
                let local = tile.native_position(cell);
                let [x, y, z] =
                    std::array::from_fn(|axis| local[axis].wrapping_add(slot.origin[axis]));
                for (mesh, material) in
                    &models[usize::from(tile.model)][usize::from(tile.quarter_turn & 3)]
                {
                    let entity = commands
                        .spawn((
                            Mesh3d(mesh.clone()),
                            MeshMaterial3d(material.clone()),
                            Name::new(format!(
                                "STG{:02}/M{map_number:03}/slot{physical}/cell{cell}",
                                source.stage
                            )),
                            Transform::from_xyz(
                                x as f32 / 2048.0,
                                -y as f32 / 2048.0,
                                -z as f32 / 2048.0,
                            ),
                        ))
                        .id();
                    visuals.entities.push(entity);
                }
            }
        }
    }
    visuals.revision = stage.revision;
    visuals.rebuild_epoch = epoch;
}

fn walking_camera(scene: Res<WalkingScene>, mut cameras: Query<&mut Transform, With<Camera3d>>) {
    let player = &scene.player;
    let rotation = scene.tables.rotation(player.controls.angles);
    let eye = rotation.apply_short(player.controls.eye.map(|value| value as i16));
    let target = rotation.apply_short(player.controls.target.map(|value| value as i16));
    let convert = |offset: [i32; 3]| {
        let position: [i32; 3] =
            std::array::from_fn(|axis| player.position[axis].wrapping_add(offset[axis]));
        Vec3::new(
            position[0] as f32 / 2048.0,
            -position[1] as f32 / 2048.0,
            -position[2] as f32 / 2048.0,
        )
    };
    for mut camera in &mut cameras {
        *camera = Transform::from_translation(convert(eye)).looking_at(convert(target), Vec3::Y);
    }
}

fn setup(
    mut commands: Commands,
    source: Res<StageAssets>,
    scene: (Option<Res<LayeredAssets>>, Option<Res<WalkingScene>>),
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    capture: Option<ResMut<Capture>>,
) {
    let (layered, walking) = scene;
    let mut builder =
        AssetSceneBuilder::new(&source.vram, &mut meshes, &mut materials, &mut images);
    let maps = if walking
        .as_ref()
        .is_some_and(|scene| scene.streamed.is_some())
    {
        Vec::new()
    } else if let Some(layered) = &layered {
        layered
            .maps
            .iter()
            .enumerate()
            .map(|(index, map)| (index as u16, map))
            .collect::<Vec<_>>()
    } else {
        vec![(source.map_number, &source.map)]
    };
    let origin = layered.as_ref().map_or([0; 3], |layered| layered.origin);
    for (map_number, map) in maps {
        let models: Vec<_> = map
            .tmd
            .objects
            .iter()
            .map(|object| {
                (0..4)
                    .map(|quarter| builder.build_object(object, quarter))
                    .collect::<Vec<_>>()
            })
            .collect();
        for cell in 0..400 {
            for tile in map.cell_chain(cell).expect("validated cell chain") {
                if !tile.present() {
                    continue;
                }
                let [x, y, z] = std::array::from_fn(|axis| {
                    tile.native_position(cell)[axis].wrapping_add(origin[axis])
                });
                for (mesh, material) in
                    &models[usize::from(tile.model)][usize::from(tile.quarter_turn & 3)]
                {
                    commands.spawn((
                        Mesh3d(mesh.clone()),
                        MeshMaterial3d(material.clone()),
                        Name::new(format!(
                            "STG{:02}/M{:03}/cell{cell}",
                            source.stage, map_number
                        )),
                        Transform::from_xyz(
                            x as f32 / 2048.0,
                            -y as f32 / 2048.0,
                            -z as f32 / 2048.0,
                        ),
                    ));
                }
            }
        }
    }
    commands.spawn((
        DirectionalLight {
            illuminance: 8000.0,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::YXZ, -0.4, -0.8, 0.0)),
    ));
    let target = if let Some(mut capture) = capture {
        let mut image = Image::new_target_texture(320, 240, TextureFormat::Rgba8UnormSrgb, None);
        image.texture_descriptor.usage |= TextureUsages::COPY_SRC;
        let handle = images.add(image);
        capture.target = Some(handle.clone());
        RenderTarget::Image(handle.into())
    } else {
        RenderTarget::default()
    };
    commands.spawn((
        Camera3d::default(),
        Msaa::Off,
        Tonemapping::None,
        DebandDither::Disabled,
        target,
        Transform::from_xyz(16.0, 13.0, 4.0).looking_at(Vec3::new(7.0, 0.8, -6.0), Vec3::Y),
    ));
}

fn capture_frame(mut commands: Commands, time: Res<Time<Real>>, mut capture: ResMut<Capture>) {
    if !capture.requested
        && capture.remaining_ticks.is_none_or(|ticks| ticks == 0)
        && time.elapsed_secs_f64() > 5.0
        && let Some(target) = &capture.target
    {
        commands
            .spawn(Screenshot::image(target.clone()))
            .observe(save_to_disk(capture.path.clone()))
            .observe(
                |_: On<ScreenshotCaptured>, mut exit: MessageWriter<AppExit>| {
                    exit.write(AppExit::Success);
                },
            );
        capture.requested = true;
    }
}

fn camera_controls(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut cameras: Query<&mut Transform, With<Camera3d>>,
) {
    let dt = time.delta_secs().min(0.1);
    let pressed = |key| f32::from(u8::from(keys.pressed(key)));
    let turn = pressed(KeyCode::ArrowLeft) - pressed(KeyCode::ArrowRight);
    let ahead = pressed(KeyCode::KeyW) - pressed(KeyCode::KeyS);
    let side = pressed(KeyCode::KeyD) - pressed(KeyCode::KeyA);
    let up = pressed(KeyCode::KeyE) - pressed(KeyCode::KeyQ);
    for mut camera in &mut cameras {
        camera.rotate_y(turn * dt * 1.6);
        let direction =
            camera.forward().as_vec3() * ahead + camera.right().as_vec3() * side + Vec3::Y * up;
        camera.translation += direction.normalize_or_zero() * dt * 4.0;
    }
}
