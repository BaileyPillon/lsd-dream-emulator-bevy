//! Inspect original stage assets while gameplay and the PS1 renderer are ported.
use bevy::{
    app::{AppExit, ScheduleRunnerPlugin},
    camera::{RenderTarget, ScalingMode, visibility::RenderLayers},
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
    GenerationAssets, StageLinkSession, StreamedStage,
    formats::{Lbd, Tix, Tmd, Vram},
    is_canonical_reference, load_day_tables, load_effect_palette, load_generation_effect_tables,
    load_generation_object_tables, load_generation_paths, load_generation_tables,
    load_layered_stage, load_link_tables, load_math_tables, sha256_hex, stage_map_path,
};
use lsd_bevy::asset_scene::AssetSceneBuilder;
use lsd_bevy::asset_scene::MeshBinding;
use lsd_bevy::box_material::{BoxMaterial, BoxMaterialPlugin, BoxUniform};
use lsd_core::{
    BoxFramebuffer, ChunkPlayer, CollisionBounds, CollisionChunk, CollisionPolicy, GenerationChild,
    GenerationPose, LinkedPlayer, MapCompletionCommand, MapEffect, PadState, PlayerControls,
    PsxMathTables, SceneCollision, SceneLinkTables, SpawnLocation,
};
use std::path::PathBuf;

#[derive(Resource)]
struct StageAssets {
    map: Lbd,
    vram: Vram,
    stage: u8,
    map_number: u16,
    texture_variant: char,
    palette_revision: u64,
}

#[derive(Resource)]
struct Capture {
    path: PathBuf,
    target: Option<Handle<Image>>,
    source: Option<Handle<Image>>,
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
    textures: Vec<(u16, u16, Handle<Image>)>,
    palette_revision: u64,
}

#[derive(Component)]
struct StreamMapMesh;

fn stream_map_scale(
    scene: Res<WalkingScene>,
    mut maps: Query<&mut Transform, With<StreamMapMesh>>,
) {
    let scale = scene
        .session
        .as_ref()
        .map_or([4096; 3], |session| session.rendered_scene_scale);
    for mut transform in &mut maps {
        transform.scale = Vec3::from_array(scale.map(|value| value as f32 / 4096.0));
    }
}

#[derive(Resource)]
struct LayeredAssets {
    maps: Vec<Lbd>,
    origin: [i32; 3],
}

#[derive(Component)]
struct GeneratedItem {
    object: usize,
    child: Option<usize>,
    sprite: bool,
}
#[derive(Resource)]
struct GeneratedVisuals {
    assets: GenerationAssets,
    epoch: u64,
    count: usize,
    entities: Vec<Entity>,
    models: Vec<Vec<(Handle<Mesh>, Handle<StandardMaterial>)>>,
}
#[derive(Component)]
struct ActorBone {
    serial: u64,
    part: usize,
}
#[derive(Component)]
struct ActorRoot(u64);
#[derive(Component)]
struct ActorMeshPart {
    serial: u64,
    part: usize,
}
struct ActorVisual {
    root: Entity,
    bones: Vec<Entity>,
    meshes: Vec<Vec<MeshBinding>>,
    bound: Vec<Option<(usize, Vec<Entity>)>>,
    parents: Vec<Option<lsd_core::AnimationParent>>,
}
#[derive(Resource, Default)]
struct ActorVisuals {
    actors: std::collections::BTreeMap<u64, ActorVisual>,
    textures: Vec<(u16, u16, Handle<Image>)>,
    palette_revision: u64,
}

#[derive(Component)]
struct GenerationLayerCamera;
#[derive(Resource, Default)]
struct GenerationLayerVisuals {
    background: Option<Handle<Image>>,
    composite: Option<Handle<BoxMaterial>>,
    last_background: Vec<u8>,
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
    let mut original_day = false;
    let mut supplied_day = 1;
    let mut supplied_actor_motion = false;
    let mut supplied_actor_query = false;
    let mut supplied_gray_contact = false;
    let mut supplied_scene_scale = false;
    let mut supplied_scene_shrink = false;
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
                "--dream"
                    | "--dream-capture"
                    | "--dream-scenario-capture"
                    | "--npc-behavior-capture"
                    | "--npc-motion-capture"
                    | "--gray-man-capture"
                    | "--scene-scale-capture"
                    | "--scene-shrink-capture"
            ) =>
        {
            original_day = true;
            supplied_actor_motion = value == "--npc-motion-capture";
            if supplied_actor_motion {
                supplied_day = 8;
            }
            supplied_actor_query = value == "--npc-behavior-capture" || supplied_actor_motion;
            supplied_gray_contact = value == "--gray-man-capture";
            supplied_scene_shrink = value == "--scene-shrink-capture";
            supplied_scene_scale = value == "--scene-scale-capture" || supplied_scene_shrink;
            walk = true;
            walk_stream = true;
            walk_links = true;
            if value != "--dream" {
                let path =
                    PathBuf::from(args.next().ok_or("--dream-capture needs a new PNG path")?);
                walk_steps = Some(
                    args.next()
                        .ok_or("--dream-capture needs a tick count")?
                        .parse::<u32>()?,
                );
                if value == "--dream-scenario-capture" {
                    supplied_day = args
                        .next()
                        .ok_or("--dream-scenario-capture needs a one-based calendar day")?
                        .parse::<i32>()?;
                    if !(1..=365).contains(&supplied_day) {
                        return Err("Supplied calendar day must be 1-365".into());
                    }
                }
                (false, Some(path))
            } else {
                (false, None)
            }
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
    let mut loaded_texture = texture.chars().next().unwrap();
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
        let body_path = "CDI/ETC/DREAME5.TMD";
        let body = Tmd::parse(&std::fs::read(root.join(body_path))?)?;
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
        if let Some(session) = &mut session {
            session.player_collision = Some(lsd_core::CollisionModel {
                bounds: Some(bounds),
                faces: body.objects[0].collision_faces(),
            });
        }
        if original_day {
            let linked = linked.as_mut().unwrap();
            let session = session.as_mut().unwrap();
            session.enable_generation(
                load_generation_tables(&executable)?,
                load_generation_paths(&executable)?,
            );
            session.enable_generated_objects(load_generation_object_tables(&executable)?);
            session.enable_generation_layers(load_generation_effect_tables(&executable)?);
            session.enable_encounters(
                lsd_assets::load_encounter_tables(&executable)?,
                lsd_assets::load_entity_properties(&executable)?,
            );
            session
                .encounters
                .as_mut()
                .unwrap()
                .enable_entity_effects(lsd_assets::load_entity_effect_definition(&executable)?);
            session
                .encounters
                .as_mut()
                .unwrap()
                .enable_special_encounter(lsd_assets::load_special_encounter_assets(
                    &executable,
                    &root,
                )?);
            session.begin_original_day(linked, load_day_tables(&executable)?);
            session.day.as_mut().unwrap().day = supplied_day - 1;
            session.tick(
                streamed.as_mut().unwrap(),
                &mut player,
                linked,
                effect.as_mut().unwrap(),
            )?;
            println!(
                "profile=original_day_preparation_prototype supplied_bootstrap_day={supplied_day} supplied_seed=1 supplied_prior_history=new_game original_generation_choices_and_actor_sound_schedule=true generated_object_channel_connected=true original_menus_gpu_audio_pending=true"
            );
        }
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
            for step_index in 0..steps {
                if let Some(streamed) = &mut streamed {
                    if supplied_scene_scale && step_index == 0 {
                        let controller = lsd_core::SceneScale::new(streamed.scene_scale_tables()?);
                        let session = session
                            .as_mut()
                            .ok_or("Scene scale capture needs a session")?;
                        session.scene_scale = Some(controller);
                        let (amount, mode) = if supplied_scene_shrink {
                            (-1, 0)
                        } else {
                            (1, 1)
                        };
                        session.scene_scale.as_mut().unwrap().request(amount, mode);
                        println!(
                            "profile=supplied_scene_scale_request amount={amount} mode={mode} natural_npc_encounter=false original_gpu_timing_pending=true"
                        );
                    }
                    streamed_step(
                        &mut player,
                        streamed,
                        &tables,
                        linked.as_mut(),
                        effect.as_mut(),
                        session.as_mut(),
                        if supplied_wall.is_some() || original_day {
                            0
                        } else {
                            0x1000
                        },
                    )?;
                    if supplied_actor_query && step_index == 0 {
                        let entity = if supplied_actor_motion { 93 } else { 5 };
                        let actor = session
                            .as_ref()
                            .and_then(|session| session.encounters.as_ref())
                            .and_then(|encounters| {
                                encounters
                                    .actors
                                    .iter()
                                    .find(|actor| actor.entity == entity)
                            })
                            .ok_or("Supplied NPC capture requires its original encounter")?;
                        let forward = i32::from(actor.properties.fields[11])
                            .abs()
                            .wrapping_shl(11);
                        let offset = lsd_core::EntityTarget {
                            position: [0; 3],
                            local_y: 0,
                            matrix: tables.rotation(player.controls.angles),
                            eye: player.controls.eye,
                            target: player.controls.target,
                        }
                        .resolve(forward)?;
                        player.position = std::array::from_fn(|axis| {
                            actor.actor.pose.position[axis].wrapping_sub(offset[axis])
                        });
                        linked
                            .as_mut()
                            .unwrap()
                            .refresh_key(&streamed.collision, player.position)?;
                        println!(
                            "profile=supplied_original_npc_proximity entity={entity} initial_actor_position={:?} supplied_player_placement=true supplied_behavior_activation=false original_npc_script_and_query_math=true original_global_cadence_gpu_spu_pending=true",
                            actor.actor.pose.position,
                        );
                    }
                    if supplied_gray_contact && step_index == 0 {
                        let target = lsd_core::EntityTarget {
                            position: player.position,
                            local_y: player.position[1],
                            matrix: tables.rotation(player.controls.angles),
                            eye: player.controls.eye,
                            target: player.controls.target,
                        };
                        let position = target.resolve(2048)?;
                        let session = session.as_mut().unwrap();
                        session.day.as_mut().unwrap().flashback_count = 10;
                        let encounters = session.encounters.as_mut().unwrap();
                        encounters.set_encounter_player(player.position, target.matrix);
                        encounters.activate_gray_man(&mut session.encounter_audio)?;
                        encounters
                            .actors
                            .iter_mut()
                            .find(|actor| actor.entity == 98)
                            .unwrap()
                            .actor
                            .translate(position, true);
                        println!(
                            "profile=supplied_original_gray_proximity supplied_actor_position=true supplied_triggered_flag=false supplied_flashback_count=10 original_action_effect_and_reset_connected=true"
                        );
                    }
                    let selected = session
                        .as_ref()
                        .and_then(|session| session.content_choice.as_ref())
                        .map_or(loaded_texture, |choice| choice.texture);
                    if display_stage != streamed.stage_id() || loaded_texture != selected {
                        display_stage = streamed.stage_id();
                        loaded_texture = selected;
                        vram = streamed.load_texture_variant(selected)?;
                    }
                    if let Some(session) = &mut session {
                        for command in session.take_generation_vram_moves() {
                            vram.copy_rectangle(command)?;
                        }
                    }
                } else if let Some(layered) = &layered {
                    player.step_in_scene(0x1000, &layered.collision, &tables)?;
                } else {
                    player.step(0x1000, &collision, &tables)?;
                }
            }
        }
        if let Some(streamed) = &mut streamed {
            streamed.complete_frame()?;
            let selected_texture = session
                .as_ref()
                .and_then(|session| session.content_choice.as_ref())
                .map_or(texture.chars().next().unwrap(), |choice| choice.texture);
            if streamed.stage_id() != display_stage || selected_texture != loaded_texture {
                display_stage = streamed.stage_id();
                vram = streamed.load_texture_variant(selected_texture)?;
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
            if let Some(encounters) = &session.encounters {
                if let Some(gray) = encounters.actors.iter().find(|actor| actor.entity == 98) {
                    println!(
                        "gray_action_ticks={} gray_action_active={} gray_flashback_count={} gray_effect_state={} gray_effect_life={} gray_effect_color={:?}",
                        gray.behavior.ticks,
                        gray.behavior.active,
                        session.day.as_ref().map_or(0, |day| day.flashback_count),
                        gray.behavior.effect.as_ref().map_or(0, |e| e.state.state),
                        gray.behavior.effect.as_ref().map_or(0, |e| e.state.life),
                        gray.behavior.effect.as_ref().map(|e| e.state.color)
                    );
                }
                println!(
                    "original_actor_channel_connected=true actor_instances={} actor_parts={} named_npc_callbacks_pending={} gray_man_activation_requests_pending={}",
                    encounters.actors.len(),
                    encounters
                        .actors
                        .iter()
                        .map(|actor| actor.actor.animation.parts.len())
                        .sum::<usize>(),
                    encounters
                        .actors
                        .iter()
                        .filter(|actor| actor.behavior_pending)
                        .count(),
                    encounters.gray_man_activation_requests
                );
                println!(
                    "original_entity_behavior_channel_connected=true implemented_named_callbacks={} active_implemented_named_callbacks={} encounter_audio_requests={} encounter_notifications_pending={}",
                    encounters
                        .actors
                        .iter()
                        .filter(
                            |actor| !actor.behavior_pending && actor.properties.callback.is_some()
                        )
                        .count(),
                    encounters
                        .actors
                        .iter()
                        .filter(|actor| !actor.behavior_pending
                            && actor.properties.callback.is_some()
                            && actor.behavior_active)
                        .count(),
                    session.encounter_audio.requests.len(),
                    encounters.pending_notifications.len()
                );
                println!(
                    "actor_body_queries_executed={} actor_body_query_contacts={}",
                    session.actor_body_queries.queries, session.actor_body_queries.contacts
                );
                println!(
                    "encounter_frame_phase={} encounter_clock_members={} encounter_frame_waiting={} encounter_frame_finished={}",
                    session.encounter_frame.phase,
                    encounters.frame_actor_order().len(),
                    session.encounter_frame.waiting,
                    session.encounter_frame.finished
                );
                println!(
                    "shared_clock_members={} shared_session_ticks={} shared_frame_trace={:?} renderer_player_snapshot_available={}",
                    session.frame_members.len(),
                    session.session_ticks,
                    session.frame_trace,
                    session.rendered_player.is_some()
                );
                if let Some(scale) = &session.scene_scale {
                    println!(
                        "scene_scale_remaining={} scene_scale_live={:?} scene_scale_rendered={:?}",
                        scale.remaining, scale.scale, session.rendered_scene_scale
                    );
                }
            }
            if supplied_actor_query {
                let entity = if supplied_actor_motion { 93 } else { 5 };
                let actor = session
                    .encounters
                    .as_ref()
                    .and_then(|encounters| {
                        encounters
                            .actors
                            .iter()
                            .find(|actor| actor.entity == entity)
                    })
                    .ok_or("NPC actor retired before supplied capture")?;
                if supplied_actor_motion {
                    println!(
                        "supplied_yokai_behavior_active={} supplied_yokai_behavior_ticks={} supplied_yokai_context_ticks={} supplied_yokai_animation_clip={:?} supplied_yokai_animation_frame={} supplied_yokai_position={:?} supplied_yokai_movement_amount={}",
                        actor.behavior.active,
                        actor.behavior.ticks,
                        actor.behavior.context.words[1],
                        actor.actor.animation.selected_clip,
                        actor.actor.animation.frame_index,
                        actor.actor.pose.position,
                        actor.actor.move_distance
                    );
                } else {
                    println!(
                        "supplied_futon_behavior_active={} supplied_futon_behavior_ticks={} supplied_futon_context_ticks={} supplied_futon_animation_frame={}",
                        actor.behavior.active,
                        actor.behavior.ticks,
                        actor.behavior.context.words[1],
                        actor.actor.animation.frame_index
                    );
                }
            }
            println!(
                "session_state={} rebuilds={} loaded_stage={} pending_day_event={:?}",
                session.policy.state, session.rebuilds, display_stage, session.pending_day_event
            );
            if let Some(day) = &session.day {
                println!(
                    "actor_journal_instance_moods={} actor_journal_instance_score={} actor_journal_current_mood={:?}",
                    day.instances.count,
                    day.instance_score,
                    day.chart()?.0
                );
                println!(
                    "original_day={} year={} stored_day={} time_limit={} result={:?} flashback_count={} score={}",
                    day.day + 1,
                    day.year,
                    day.stored_day,
                    linked.as_ref().unwrap().links.time_limit,
                    session.finished,
                    day.flashback_count,
                    day.total_unlock_score
                );
            }
            if let (Some(generation), Some(choice)) = (&session.generation, &session.content_choice)
            {
                println!(
                    "generation_kind={} original_scene_style={:?} selected_texture={} selected_music={} generation_tick={} active_generation_actors={} generation_objects={} generation_gradient_layers={} generation_environment_layer={} generation_object_preview_connected=true generation_layer_preview_connected=true palette_copy_channel_connected=true original_gpu_precision_and_audio_playback_pending=true",
                    generation.generation_type,
                    generation.style,
                    choice.texture,
                    choice.music_index,
                    generation.timer,
                    generation
                        .actors
                        .iter()
                        .filter(|actor| actor.active)
                        .count(),
                    generation.structures.len(),
                    generation.layers.gradient.len(),
                    generation.layers.environment.is_some()
                );
                println!(
                    "generation_foreground_box_packets={:?}",
                    generation
                        .layers
                        .environment
                        .and_then(|layer| layer.gpu_box_packet([320, 240]))
                        .map(|packet| packet.words())
                        .into_iter()
                        .collect::<Vec<_>>()
                );
            }
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
    let selected_texture = walking
        .as_ref()
        .and_then(|scene| scene.session.as_ref())
        .and_then(|session| session.content_choice.as_ref())
        .map_or(texture.chars().next().unwrap(), |choice| choice.texture);
    let mut app = App::new();
    if original_day {
        app.init_resource::<ActorVisuals>()
            .add_systems(Update, scene_actors);
        app.insert_resource(GeneratedVisuals {
            assets: GenerationAssets::load(&root, &executable)?,
            epoch: u64::MAX,
            count: usize::MAX,
            entities: Vec::new(),
            models: Vec::new(),
        })
        .insert_resource(GenerationLayerVisuals::default())
        .add_systems(Update, generated_objects.after(walking_camera))
        .add_systems(Update, generation_layers);
    }
    if walk_stream {
        app.insert_resource(StreamVisuals {
            revision: u64::MAX,
            rebuild_epoch: u64::MAX,
            entities: Vec::new(),
            models: Default::default(),
            textures: Vec::new(),
            palette_revision: u64::MAX,
        })
        .add_systems(Update, (stream_meshes, stream_map_scale).chain());
    }
    if let Some(layered) = layered_assets {
        app.insert_resource(layered);
    }
    if let Some(walking) = walking {
        app.insert_resource(walking)
            .insert_resource(Time::<Fixed>::from_hz(15.0))
            .add_systems(FixedUpdate, walking_controls)
            .add_systems(Update, (walking_camera, generation_environment));
    }
    app.insert_resource(StageAssets {
        map,
        vram,
        stage: display_stage,
        map_number,
        texture_variant: selected_texture,
        palette_revision: 0,
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
        if path.exists() || original_day && path.with_extension("source.png").exists() {
            return Err("Capture output already exists".into());
        }
        app.insert_resource(Capture {
            path,
            target: None,
            source: None,
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
    if original_day {
        app.add_plugins(BoxMaterialPlugin);
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
        let selected_texture = session
            .as_ref()
            .and_then(|session| session.content_choice.as_ref())
            .map_or(source.texture_variant, |choice| choice.texture);
        if source.stage != streamed.stage_id() || source.texture_variant != selected_texture {
            source.stage = streamed.stage_id();
            source.texture_variant = selected_texture;
            source.vram = streamed
                .load_texture_variant(source.texture_variant)
                .expect("validated original destination texture");
        }
        if let Some(session) = session {
            let moves = session.take_generation_vram_moves();
            if !moves.is_empty() {
                for command in moves {
                    source
                        .vram
                        .copy_rectangle(command)
                        .expect("bounded original palette strip copy");
                }
                source.palette_revision = source.palette_revision.wrapping_add(1);
            }
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
    mut linked: Option<&mut LinkedPlayer>,
    mut effect: Option<&mut MapEffect>,
    mut session: Option<&mut StageLinkSession>,
    buttons: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    if let (Some(linked), Some(effect), Some(session)) = (
        linked.as_deref_mut(),
        effect.as_deref_mut(),
        session.as_deref_mut(),
    ) {
        session.step_frame(stage, linked, player, tables, effect, buttons)?;
        for event in &session.events {
            println!("session_command={event:?}");
        }
        return Ok(());
    }
    stage.complete_frame()?;
    if let Some(linked) = linked {
        let mut effect = effect;
        let mut session = session;
        if let (Some(session), Some(effect)) = (session.as_deref_mut(), effect.as_deref_mut()) {
            session.tick(stage, player, linked, effect)?;
            for event in &session.events {
                println!("session_command={event:?}");
            }
            if session.finished.is_some() || session.pending_day_event.is_some() {
                player.controls.input_locked = true;
                return Ok(());
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
        if let Some(session) = session.as_deref_mut() {
            if let (Some(day), Some(content)) = (&mut session.day, &session.day_tables) {
                linked.step_with_clock(
                    player,
                    &stage.collision,
                    tables,
                    ready,
                    buttons,
                    Some((day, content)),
                )?;
            } else {
                linked.step(player, &stage.collision, tables, ready, buttons)?;
            }
        } else {
            linked.step(player, &stage.collision, tables, ready, buttons)?;
        }
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
        let previous_chunk = linked.keys.current.chunk;
        linked.refresh_key(&stage.collision, player.position)?;
        if previous_chunk != linked.keys.current.chunk
            && let Some(session) = session.as_deref_mut()
        {
            session.log_current_chunk(linked)?;
        }
        if let Some(session) = session {
            if let Some(effect) = effect {
                session.tick_encounters(stage, linked, player, tables, effect)?;
            }
            session.tick_generation_actors(stage, linked, &player.controls)?;
        }
    } else {
        player.step_in_scene(buttons, &stage.collision, tables)?;
        stage.update_for_position(player.position)?;
    }
    Ok(())
}

fn generation_environment(scene: Res<WalkingScene>, mut clear: ResMut<ClearColor>) {
    if let Some(style) = scene
        .session
        .as_ref()
        .and_then(|session| session.generation.as_ref())
        .and_then(|generation| generation.style)
    {
        let [r, g, b] = scene
            .session
            .as_ref()
            .and_then(|s| s.generation.as_ref())
            .and_then(|g| g.layers.stage_color)
            .unwrap_or(style.scene_color);
        clear.0 = Color::srgb_u8(r, g, b);
    }
}

// Presentation adapter. Original PS1 projection, draw order and GPU precision
// remain separate requirements; the logical poses above are native-checked.
fn generation_layers(
    scene: Res<WalkingScene>,
    mut visuals: ResMut<GenerationLayerVisuals>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<BoxMaterial>>,
) {
    let Some(generation) = scene
        .session
        .as_ref()
        .and_then(|session| session.generation.as_ref())
    else {
        return;
    };
    let Some(style) = generation.style else {
        return;
    };
    let clear = generation.layers.stage_color.unwrap_or(style.scene_color);
    let mut framebuffer = BoxFramebuffer::new(320, 240, clear);
    for layer in &generation.layers.gradient {
        if let Some(packet) = layer.gpu_box_packet([320, 240]) {
            framebuffer.draw_box(packet);
        }
    }
    let rgba = framebuffer.rgba();
    if visuals.last_background != rgba {
        if let Some(handle) = &visuals.background
            && let Some(mut image) = images.get_mut(handle)
        {
            image.data = Some(rgba.clone());
        }
        visuals.last_background = rgba;
    }
    if let Some(handle) = &visuals.composite
        && let Some(mut material) = materials.get_mut(handle)
    {
        let mut packets: Vec<_> = generation
            .layers
            .environment
            .and_then(|layer| layer.gpu_box_packet([320, 240]))
            .into_iter()
            .collect();
        if let Some(encounters) = scene
            .session
            .as_ref()
            .and_then(|session| session.encounters.as_ref())
        {
            packets.extend(encounters.entity_effect_packets([320, 240]));
        }
        material.boxes.set_packets(&packets);
    }
}

fn generated_pose(pose: GenerationPose) -> Transform {
    let [x, y, z] = pose.position.map(|v| v as f32 / 2048.0);
    let [rx, ry, rz] = pose
        .angles
        .map(|v| f32::from(v) * std::f32::consts::TAU / 4096.0);
    Transform {
        translation: Vec3::new(x, -y, -z),
        rotation: Quat::from_euler(EulerRot::XYZ, rx, -ry, -rz),
        scale: Vec3::from_array(pose.scale.map(|v| v as f32 / 4096.0)),
    }
}

type ActorNodes<'w, 's> = (
    Query<
        'w,
        's,
        (
            &'static ActorBone,
            &'static mut Transform,
            &'static mut Visibility,
        ),
        (Without<ActorRoot>, Without<ActorMeshPart>),
    >,
    Query<'w, 's, (&'static ActorRoot, &'static mut Transform), (Without<ActorBone>,)>,
    Query<'w, 's, (&'static ActorMeshPart, &'static mut Visibility)>,
);
fn scene_actors(
    mut commands: Commands,
    scene: Res<WalkingScene>,
    source: Res<StageAssets>,
    mut visuals: ResMut<ActorVisuals>,
    assets: GeneratedAssetStores<'_>,
    nodes: ActorNodes<'_, '_>,
) {
    let Some(encounters) = scene
        .session
        .as_ref()
        .and_then(|session| session.encounters.as_ref())
    else {
        return;
    };
    let (mut meshes, mut materials, mut images) = assets;
    if visuals.palette_revision != source.palette_revision {
        for (page, clut, handle) in &visuals.textures {
            if (clut >> 6) & 511 >= 496
                && let Some(mut image) = images.get_mut(handle)
            {
                image.data = Some(
                    source
                        .vram
                        .texture_rgba(*page, *clut)
                        .expect("validated actor palette texture"),
                );
            }
        }
        visuals.palette_revision = source.palette_revision;
    }
    let retired: Vec<_> = visuals
        .actors
        .keys()
        .copied()
        .filter(|serial| {
            !encounters
                .actors
                .iter()
                .any(|actor| actor.serial == *serial)
        })
        .collect();
    for serial in retired {
        let visual = visuals.actors.remove(&serial).unwrap();
        commands.entity(visual.root).despawn();
    }
    if visuals.actors.is_empty() {
        visuals.textures.clear();
    }
    for actor in &encounters.actors {
        if !visuals.actors.contains_key(&actor.serial) {
            let root = commands
                .spawn((
                    ActorRoot(actor.serial),
                    Name::new(format!(
                        "Original entity {} slot {} map {}",
                        actor.entity, actor.slot, actor.map
                    )),
                    generated_pose(actor.actor.pose),
                    Visibility::Inherited,
                ))
                .id();
            let bones = actor
                .actor
                .animation
                .parts
                .iter()
                .enumerate()
                .map(|(part, data)| {
                    commands
                        .spawn((
                            ActorBone {
                                serial: actor.serial,
                                part,
                            },
                            Name::new(format!("Entity {} part {}", actor.entity, data.part_type)),
                            actor_part_pose(data),
                            if data.parent.is_none() {
                                Visibility::Hidden
                            } else {
                                Visibility::Inherited
                            },
                        ))
                        .id()
                })
                .collect::<Vec<_>>();
            for (part, data) in actor.actor.animation.parts.iter().enumerate() {
                let parent = match data.parent {
                    Some(lsd_core::AnimationParent::Part(owner)) => bones[owner],
                    _ => root,
                };
                commands.entity(bones[part]).insert(ChildOf(parent));
            }
            let mut builder =
                AssetSceneBuilder::new(&source.vram, &mut meshes, &mut materials, &mut images);
            let model_meshes = actor
                .resource
                .tmd
                .objects
                .iter()
                .map(|object| builder.build_object(object, 0))
                .collect();
            visuals.textures.extend(builder.take_texture_bindings());
            visuals.actors.insert(
                actor.serial,
                ActorVisual {
                    root,
                    bones,
                    meshes: model_meshes,
                    bound: vec![None; actor.actor.animation.parts.len()],
                    parents: actor
                        .actor
                        .animation
                        .parts
                        .iter()
                        .map(|part| part.parent)
                        .collect(),
                },
            );
        }
        let visual = visuals.actors.get_mut(&actor.serial).unwrap();
        for (part, data) in actor.actor.animation.parts.iter().enumerate() {
            if visual.parents[part] != data.parent {
                let parent = match data.parent {
                    Some(lsd_core::AnimationParent::Part(owner)) => visual.bones[owner],
                    _ => visual.root,
                };
                commands.entity(visual.bones[part]).insert(ChildOf(parent));
                visual.parents[part] = data.parent;
            }
            if visual.bound[part].as_ref().map(|binding| binding.0) != data.primitive {
                if let Some((_, old_meshes)) = visual.bound[part].take() {
                    for entity in old_meshes {
                        commands.entity(entity).despawn();
                    }
                }
                let Some(primitive) = data.primitive else {
                    continue;
                };
                let mut entities = Vec::new();
                for (mesh, material) in &visual.meshes[primitive] {
                    let entity = commands
                        .spawn((
                            ActorMeshPart {
                                serial: actor.serial,
                                part,
                            },
                            Mesh3d(mesh.clone()),
                            MeshMaterial3d(material.clone()),
                            Transform::default(),
                            ChildOf(visual.bones[part]),
                            if data.attributes & (1 << 31) != 0 {
                                Visibility::Hidden
                            } else {
                                Visibility::Inherited
                            },
                        ))
                        .id();
                    entities.push(entity);
                }
                visual.bound[part] = Some((primitive, entities));
            }
        }
    }
    let (mut bones, mut roots, mut visible_meshes) = nodes;
    for (bone, mut transform, mut visibility) in &mut bones {
        if let Some(actor) = encounters
            .actors
            .iter()
            .find(|actor| actor.serial == bone.serial)
        {
            let part = &actor.actor.animation.parts[bone.part];
            *transform = actor_part_pose(part);
            *visibility = if part.parent.is_none() {
                Visibility::Hidden
            } else {
                Visibility::Inherited
            };
        }
    }
    for (root, mut transform) in &mut roots {
        if let Some(actor) = encounters
            .actors
            .iter()
            .find(|actor| actor.serial == root.0)
        {
            *transform = generated_pose(actor.actor.pose);
        }
    }
    for (mesh, mut visibility) in &mut visible_meshes {
        if let Some(actor) = encounters
            .actors
            .iter()
            .find(|actor| actor.serial == mesh.serial)
        {
            *visibility = if actor.actor.animation.parts[mesh.part].attributes & (1 << 31) != 0 {
                Visibility::Hidden
            } else {
                Visibility::Inherited
            };
        }
    }
}
fn actor_part_pose(part: &lsd_core::AnimationPart) -> Transform {
    generated_pose(GenerationPose {
        position: part.coordinate_translation,
        angles: part.angles,
        scale: part.scale,
    })
}

type GeneratedAssetStores<'w> = (
    ResMut<'w, Assets<Mesh>>,
    ResMut<'w, Assets<StandardMaterial>>,
    ResMut<'w, Assets<Image>>,
);
type GeneratedCameras<'w, 's> = Query<
    'w,
    's,
    (&'static Transform, &'static Projection),
    (
        With<Camera3d>,
        Without<GeneratedItem>,
        Without<GenerationLayerCamera>,
    ),
>;

fn generated_objects(
    mut commands: Commands,
    scene: Res<WalkingScene>,
    mut visuals: ResMut<GeneratedVisuals>,
    assets: GeneratedAssetStores<'_>,
    mut nodes: Query<(&GeneratedItem, &mut Transform, &mut Visibility)>,
    cameras: GeneratedCameras<'_, '_>,
) {
    let Some(session) = &scene.session else {
        return;
    };
    let Some(generation) = &session.generation else {
        return;
    };
    let (mut meshes, mut materials, mut images) = assets;
    if visuals.epoch != session.rebuilds || visuals.count != generation.structures.len() {
        for entity in visuals.entities.drain(..) {
            commands.entity(entity).despawn();
        }
        if visuals.models.is_empty() {
            let GeneratedVisuals { assets, models, .. } = &mut *visuals;
            let mut builder =
                AssetSceneBuilder::new(&assets.vram, &mut meshes, &mut materials, &mut images);
            *models = assets
                .model
                .objects
                .iter()
                .map(|object| builder.build_object(object, 0))
                .collect();
        }
        for (index, object) in generation.structures.iter().enumerate() {
            if let Some(model) = object.model_index {
                for (mesh, material) in visuals.models[model as usize].clone() {
                    let entity = commands
                        .spawn((
                            Mesh3d(mesh),
                            MeshMaterial3d(material),
                            generated_pose(object.pose),
                            Visibility::Inherited,
                            GeneratedItem {
                                object: index,
                                child: None,
                                sprite: false,
                            },
                            Name::new(format!("Original generation model {index}")),
                        ))
                        .id();
                    visuals.entities.push(entity);
                }
            }
            for (child_index, child) in object.children.iter().enumerate() {
                match child {
                    GenerationChild::Model { pose, model_index } => {
                        let transform = Transform::from_matrix(
                            generated_pose(object.pose).to_matrix()
                                * generated_pose(*pose).to_matrix(),
                        );
                        for (mesh, material) in visuals.models[*model_index as usize].clone() {
                            let entity = commands
                                .spawn((
                                    Mesh3d(mesh),
                                    MeshMaterial3d(material),
                                    transform,
                                    Visibility::Inherited,
                                    GeneratedItem {
                                        object: index,
                                        child: Some(child_index),
                                        sprite: false,
                                    },
                                    Name::new(format!(
                                        "Original generation child {index}/{child_index}"
                                    )),
                                ))
                                .id();
                            visuals.entities.push(entity);
                        }
                    }
                    GenerationChild::Sprite(sprite) => {
                        let (width, height, rgba) = visuals
                            .assets
                            .sprite_rgba(sprite)
                            .expect("Canonical generation sprite texture");
                        let mut image = Image::new(
                            bevy::render::render_resource::Extent3d {
                                width,
                                height,
                                depth_or_array_layers: 1,
                            },
                            bevy::render::render_resource::TextureDimension::D2,
                            rgba,
                            TextureFormat::Rgba8UnormSrgb,
                            default(),
                        );
                        image.sampler = bevy::image::ImageSampler::nearest();
                        let texture = images.add(image);
                        let material = materials.add(StandardMaterial {
                            base_color_texture: Some(texture),
                            unlit: true,
                            alpha_mode: AlphaMode::Blend,
                            cull_mode: None,
                            ..default()
                        });
                        let entity = commands
                            .spawn((
                                Mesh3d(meshes.add(Rectangle::new(1.0, 1.0))),
                                MeshMaterial3d(material),
                                Transform::default(),
                                Visibility::Hidden,
                                GeneratedItem {
                                    object: index,
                                    child: Some(child_index),
                                    sprite: true,
                                },
                                Name::new(format!(
                                    "Original generation sprite {index}/{child_index}"
                                )),
                            ))
                            .id();
                        visuals.entities.push(entity);
                    }
                }
            }
        }
        visuals.epoch = session.rebuilds;
        visuals.count = generation.structures.len();
    }
    let camera = cameras.iter().next();
    for (node, mut transform, mut visibility) in &mut nodes {
        let Some(object) = generation.structures.get(node.object) else {
            *visibility = Visibility::Hidden;
            continue;
        };
        let root = generated_pose(object.pose).to_matrix();
        if node.sprite {
            let Some(GenerationChild::Sprite(sprite)) =
                node.child.and_then(|i| object.children.get(i))
            else {
                continue;
            };
            let Some((camera, Projection::Perspective(projection))) = camera else {
                continue;
            };
            let position = root.transform_point3(generated_pose(sprite.pose).translation);
            let depth = -camera
                .compute_affine()
                .inverse()
                .transform_point3(position)
                .z;
            if depth <= 0.0 || sprite.flags & (1 << 31) != 0 {
                *visibility = Visibility::Hidden;
                continue;
            }
            let pixel = 2.0 * depth * (projection.fov * 0.5).tan() / 240.0;
            *transform = Transform {
                translation: position,
                rotation: camera.rotation
                    * Quat::from_rotation_z(-(sprite.rotation as f32 / 4096.0).to_radians()),
                scale: Vec3::new(
                    f32::from(sprite.rectangle[2]) * f32::from(sprite.scale[0]) / 4096.0 * pixel,
                    f32::from(sprite.rectangle[4]) * f32::from(sprite.scale[1]) / 4096.0 * pixel,
                    1.0,
                ),
            };
            *visibility = Visibility::Inherited;
        } else {
            *transform = if let Some(GenerationChild::Model { pose, .. }) =
                node.child.and_then(|i| object.children.get(i))
            {
                Transform::from_matrix(root * generated_pose(*pose).to_matrix())
            } else {
                generated_pose(object.pose)
            };
            *visibility = Visibility::Inherited;
        }
    }
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
    if visuals.palette_revision != source.palette_revision {
        for (page, clut, handle) in &visuals.textures {
            if (clut >> 6) & 511 >= 496
                && let Some(mut image) = images.get_mut(handle)
            {
                image.data = Some(
                    source
                        .vram
                        .texture_rgba(*page, *clut)
                        .expect("validated palette texture"),
                );
            }
        }
        visuals.palette_revision = source.palette_revision;
    }
    if visuals.revision == stage.revision && visuals.rebuild_epoch == epoch {
        return;
    }
    for entity in visuals.entities.drain(..) {
        commands.entity(entity).despawn();
    }
    if visuals.rebuild_epoch != epoch {
        visuals.models.clear();
        visuals.textures.clear();
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
                            StreamMapMesh,
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
    visuals.textures.extend(builder.take_texture_bindings());
    visuals.revision = stage.revision;
    visuals.rebuild_epoch = epoch;
}

fn walking_camera(
    scene: Res<WalkingScene>,
    mut cameras: Query<&mut Transform, (With<Camera3d>, Without<GenerationLayerCamera>)>,
) {
    let player = scene
        .session
        .as_ref()
        .filter(|session| session.rendered_rebuild == session.rebuilds)
        .and_then(|session| session.rendered_player.as_ref())
        .unwrap_or(&scene.player);
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
    assets: GeneratedAssetStores<'_>,
    capture: Option<ResMut<Capture>>,
    layer_assets: (
        Option<ResMut<GenerationLayerVisuals>>,
        Option<ResMut<Assets<BoxMaterial>>>,
    ),
) {
    let (mut meshes, mut materials, mut images) = assets;
    let (mut layer_visuals, mut box_materials) = layer_assets;
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
    let mut capture = capture;
    let target = if let Some(capture) = capture.as_mut() {
        let mut image = Image::new_target_texture(320, 240, TextureFormat::Rgba8UnormSrgb, None);
        image.texture_descriptor.usage |= TextureUsages::COPY_SRC;
        let handle = images.add(image);
        capture.target = Some(handle.clone());
        RenderTarget::Image(handle.into())
    } else {
        RenderTarget::default()
    };
    let layers_enabled = walking
        .as_ref()
        .and_then(|scene| scene.session.as_ref())
        .is_some_and(|session| session.generation_effect_tables.is_some());
    let world_target = if layers_enabled {
        let mut image = Image::new_target_texture(320, 240, TextureFormat::Rgba8UnormSrgb, None);
        image.texture_descriptor.usage |= TextureUsages::COPY_SRC;
        let world = images.add(image);
        if let Some(capture) = capture.as_mut() {
            capture.source = Some(world.clone());
        }
        let framebuffer = BoxFramebuffer::new(320, 240, [0; 3]);
        let mut background_image = Image::new(
            bevy::render::render_resource::Extent3d {
                width: 320,
                height: 240,
                depth_or_array_layers: 1,
            },
            bevy::render::render_resource::TextureDimension::D2,
            framebuffer.rgba(),
            TextureFormat::Rgba8UnormSrgb,
            bevy::asset::RenderAssetUsages::default(),
        );
        background_image.sampler = bevy::image::ImageSampler::nearest();
        let background = images.add(background_image);
        let material = materials.add(StandardMaterial {
            base_color_texture: Some(background.clone()),
            unlit: true,
            ..default()
        });
        commands.spawn((
            RenderLayers::layer(1),
            Mesh3d(meshes.add(Rectangle::new(320.0, 240.0))),
            MeshMaterial3d(material),
            Transform::default(),
        ));
        let composite = box_materials
            .as_mut()
            .expect("box material plugin")
            .add(BoxMaterial {
                boxes: BoxUniform::default(),
                scene: world.clone(),
            });
        commands.spawn((
            RenderLayers::layer(2),
            Mesh3d(meshes.add(Rectangle::new(320.0, 240.0))),
            MeshMaterial3d(composite.clone()),
            Transform::default(),
        ));
        let visuals = layer_visuals.as_mut().expect("generation layer resources");
        visuals.background = Some(background);
        visuals.composite = Some(composite);
        for (layer, order, camera_target) in [
            (1, -1, RenderTarget::Image(world.clone().into())),
            (2, 1, target.clone()),
        ] {
            commands.spawn((
                GenerationLayerCamera,
                Camera3d::default(),
                Camera { order, ..default() },
                Projection::Orthographic(OrthographicProjection {
                    scaling_mode: ScalingMode::Fixed {
                        width: 320.0,
                        height: 240.0,
                    },
                    ..OrthographicProjection::default_3d()
                }),
                RenderLayers::layer(layer),
                Msaa::Off,
                Tonemapping::None,
                DebandDither::Disabled,
                camera_target,
                Transform::from_xyz(0.0, 0.0, 100.0),
            ));
        }
        RenderTarget::Image(world.into())
    } else {
        target
    };
    commands.spawn((
        Camera3d::default(),
        Camera {
            clear_color: if layers_enabled {
                ClearColorConfig::None
            } else {
                ClearColorConfig::Default
            },
            ..default()
        },
        Msaa::Off,
        Tonemapping::None,
        DebandDither::Disabled,
        world_target,
        Transform::from_xyz(16.0, 13.0, 4.0).looking_at(Vec3::new(7.0, 0.8, -6.0), Vec3::Y),
    ));
}

fn capture_frame(
    mut commands: Commands,
    time: Res<Time<Real>>,
    mut capture: ResMut<Capture>,
    actor_roots: Query<(), With<ActorRoot>>,
    actor_bones: Query<(), With<ActorBone>>,
    actor_meshes: Query<&InheritedVisibility, With<ActorMeshPart>>,
) {
    if !capture.requested
        && capture.remaining_ticks.is_none_or(|ticks| ticks == 0)
        && time.elapsed_secs_f64() > 5.0
        && let Some(target) = &capture.target
    {
        println!(
            "bevy_actor_render_roots={} bevy_actor_render_bones={} bevy_actor_render_meshes={} inherited_visible_actor_meshes={}",
            actor_roots.iter().len(),
            actor_bones.iter().len(),
            actor_meshes.iter().len(),
            actor_meshes
                .iter()
                .filter(|visibility| visibility.get())
                .count()
        );
        if let Some(source) = &capture.source {
            commands
                .spawn(Screenshot::image(source.clone()))
                .observe(save_to_disk(capture.path.with_extension("source.png")));
        }
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
    mut cameras: Query<&mut Transform, (With<Camera3d>, Without<GenerationLayerCamera>)>,
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
