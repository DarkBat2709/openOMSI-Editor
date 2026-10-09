//! Opening a map: the season, the world, the spawn point.

use super::*;

/// The map's season for these arguments (kind, texture folder): `--season` (with its phase),
/// else the date
/// as the map's season table has it; a snow weather puts the map into its snow textures
/// whatever the calendar says - with snow on the road (`[snowOnRoad]`) into the snowy
/// roads of `WinterSnowfall` too (see `omsi_texture::season_chain`).
pub(crate) fn season_folder(args: &Args, global: &omsi_map::GlobalCfg) -> (i32, Option<String>) {
    let weather = load_weather(args);
    season_folder_on(args, global, start_clock(args).day_of_year, weather.snow, weather.snow_on_road)
}

/// [`season_folder`] on day `day_of_year` in weather that is snowy or not (the date moves on
/// at midnight, the weather changes).
pub(crate) fn season_folder_on(args: &Args, global: &omsi_map::GlobalCfg, day_of_year: i32, snow: bool, snow_on_road: bool) -> (i32, Option<String>) {
    let mut kind = global.season_kind(day_of_year);
    // a chosen season: the look that prevails in its phase (see `season_phase`)
    if let Some(choice) = args.season.as_deref().and_then(crate::season_phase::SeasonChoice::parse) {
        kind = choice.kind();
    }
    let mut folder = omsi_map::global::Season::folder(kind).map(|f| f.to_string());
    if snow {
        folder = Some(if snow_on_road { "WinterSnowfall" } else { "WinterSnow" }.to_string());
    }
    (kind, folder)
}

/// Open the map (global.cfg, season, chrono) and work out where the view starts: the
/// camera, and the point the tiles are loaded around (the spawn entry point when a vehicle
/// is requested).
pub(crate) fn open_world(args: &Args) -> Result<(World, Camera, DVec3)> {
    let map_cfg = omsi_cfg::resolve_path(&args.root, &args.map);
    let date = start_clock(args).date_code();
    let world = match crate::duty_start::take_preopened(&map_cfg, date) {
        Some(w) => w,
        None => World::open(&args.root, &map_cfg, date)?,
    };
    *world.start_clock.lock() = start_clock(args);
    {
        let (kind, folder) = season_folder(args, &world.global);
        log::info!(
            "season: day {} -> kind {kind} (texture folder {:?})",
            start_clock(args).day_of_year,
            folder
        );
        omsi_texture::set_season_folder(folder);
        // the plants of a phase between two looks (none by date: the map's own table)
        let mix = args.season.as_deref().and_then(crate::season_phase::SeasonChoice::parse).and_then(|c| c.mix());
        if let Some(m) = &mix {
            log::info!("season: plants mixed between {:?} and {:?}, {:.0} % in the second", m.from, m.to, m.share * 100.0);
        }
        omsi_texture::set_season_mix(mix);
    }
    let camera = match &args.cam {
        Some(c) => parse_cam(c)?,
        None => {
            let mut camera = default_camera(&world);
            if args.editor {
                let key = omsi_launcher_lib::editor_views::key(&args.root, &args.map);
                let dir = omsi_launcher_lib::data_dir().join("editor-views");
                camera.pitch = -30.0;
                let saved = omsi_launcher_lib::editor_views::load(&dir, &key);
                let remembered = saved.is_some();
                if let Some(view) = saved {
                    camera.position = DVec3::from_array(view.position);
                    camera.yaw = view.yaw;
                    camera.pitch = view.pitch;
                    camera.roll = view.roll;
                    camera.fov_deg = view.fov;
                    log::info!("Editor: letzte Kameraposition für {} wiederhergestellt", args.map);
                }
                editor_start_above_ground(&world, &mut camera, remembered);
            }
            camera
        },
    };
    // centre the loaded area on the spawn point (a requested vehicle's entry point, unless a
    // camera is given too)
    let spawn = spawn_point(args, Some(&world));
    if let (None, true, None) = (spawn, args.bus.is_some(), &args.spawn) {
        if let Some(ep) = world
            .global
            .entry_points
            .get(args.entry)
            .or(world.global.entry_points.first())
        {
            log::warn!(
                "entry point {} \"{}\": object {} is not in the map",
                ep.index,
                ep.name,
                ep.object_id
            );
        }
    }
    let center = match spawn {
        Some(p) if args.spawn.is_some() || args.cam.is_none() => p,
        _ => camera.position,
    };
    Ok((world, camera, center))
}

/// Where the vehicle starts: `--spawn`, else the requested bus's entry point (None without
/// either, or when the entry point's object is not in the map). The height is not known.
pub(crate) fn spawn_point(args: &Args, world: Option<&World>) -> Option<DVec3> {
    if let Some(sp) = &args.spawn {
        let v: Vec<f64> = sp
            .split(',')
            .filter_map(|x| x.trim().parse().ok())
            .collect();
        if v.len() >= 2 {
            return Some(DVec3::new(v[0], v[1], 0.0));
        }
    }
    args.bus.as_ref()?;
    let world = world?;
    let ep = world
        .global
        .entry_points
        .get(args.entry)
        .or(world.global.entry_points.first())?;
    // the entry points of tiles that are not loaded come from the map index
    world.index();
    world.entry_point_place(ep).map(|p| p.0)
}

pub(crate) fn load_world(args: &Args, renderer: &Renderer, scene: &mut Scene) -> Result<(World, Camera)> {
    let t0 = Instant::now();
    let (world, camera, center) = open_world(args)?;
    let tile_of = |p: DVec3| {
        (
            (p.x / omsi_map::tile_size()).floor() as i32,
            (p.y / omsi_map::tile_size()).floor() as i32,
        )
    };
    let radius = if args.all {
        None
    } else {
        Some(
            args.radius
                .unwrap_or(if args.offscreen.is_some() { 2 } else { 100 }),
        )
    };
    let mut tiles = world.select_tiles(Some(tile_of(center)), radius);
    // the bus stands on ground wherever the camera looks
    if let (Some(spawn), Some(_)) = (spawn_point(args, Some(&world)), radius) {
        for t in world.select_tiles(Some(tile_of(spawn)), radius) {
            if !tiles.iter().any(|x| (x.0, x.1) == (t.0, t.1)) {
                tiles.push(t);
            }
        }
    }
    log::info!("map {}: loading {} tiles", world.global.name, tiles.len());
    let stats = world.build_scene(renderer, scene, &tiles)?;
    // OMSI_CHURN=x,y (a point a few tiles away): the tiles around that point are loaded,
    // the start area unloaded, the far tiles unloaded and the start area loaded again, with
    // the per-draw buffers uploaded in between as a window's frames do - the picture then
    // shows the start area drawn from recycled GPU slots, as after a drive away and back.
    let churn = omsi_cfg::flags::OMSI_CHURN.var().and_then(|v| {
        let (a, b) = v.split_once(',')?;
        Some((a.trim().parse::<f64>().ok()?, b.trim().parse::<f64>().ok()?))
    });
    if let Some((fx, fy)) = churn {
        let far: Vec<_> = world
            .select_tiles(
                Some(tile_of(DVec3::new(fx, fy, 0.0))),
                Some(radius.unwrap_or(1).min(2)),
            )
            .into_iter()
            .filter(|t| !tiles.iter().any(|x| (x.0, x.1) == (t.0, t.1)))
            .collect();
        renderer.prepare(scene);
        world.build_scene(renderer, scene, &far)?;
        renderer.prepare(scene);
        for t in &tiles {
            world.unload_tile(renderer, scene, (t.0, t.1), None);
        }
        world.trim_object_types();
        world.refresh_tile_lists();
        renderer.prepare(scene);
        log::info!(
            "churn: start area unloaded, {} far tiles loaded: {}",
            far.len(),
            world.gpu_summary(scene)
        );
        for t in &far {
            world.unload_tile(renderer, scene, (t.0, t.1), None);
        }
        world.trim_object_types();
        world.refresh_tile_lists();
        renderer.prepare(scene);
        world.build_scene(renderer, scene, &tiles)?;
        log::info!(
            "churn: far tiles unloaded, the start area of {} tiles loaded again: {}",
            tiles.len(),
            world.gpu_summary(scene)
        );
    }
    log::info!(
        "loaded in {:.2}s: {} tiles, {} objects + {} trees ({} types, {} unresolved), {} splines ({} types), {} textures; {} spline attachment rows, {} attached objects ({} without their parent), {} empty parking spaces",
        t0.elapsed().as_secs_f32(),
        stats.tiles,
        stats.objects,
        stats.trees,
        stats.object_types,
        stats.failed_objects,
        stats.splines,
        stats.spline_types,
        stats.textures,
        stats.rows,
        stats.attached,
        stats.unattached,
        stats.empty_spaces
    );
    Ok((world, camera))
}

/// Read the starting tile before streaming so elevated maps do not start underground.
fn editor_start_above_ground(world: &World, camera: &mut Camera, remembered: bool) {
    let size = omsi_map::tile_size();
    let xy = ((camera.position.x / size).floor() as i32, (camera.position.y / size).floor() as i32);
    let tile = world.global.tiles.iter().find(|t| (t.x, t.y) == xy).or_else(|| {
        world.global.tiles.iter().min_by(|a, b| {
            let distance = |t: &omsi_map::global::MapTileRef| {
                let dx = (t.x as f64 + 0.5) * size - camera.position.x;
                let dy = (t.y as f64 + 0.5) * size - camera.position.y;
                dx * dx + dy * dy
            };
            distance(a).total_cmp(&distance(b))
        })
    });
    let ground = tile.and_then(|t| {
        if (t.x, t.y) != xy {
            camera.position.x = (t.x as f64 + 0.5) * size;
            camera.position.y = (t.y as f64 + 0.5) * size;
        }
        let path = omsi_cfg::resolve_path(world.global.dir(), &t.file);
        let parsed = crate::tiles::read_tile(&path, &world.chrono_dirs.read());
        let terrain_path = parsed.as_ref().map(|t| crate::tiles::terrain_file(t, &path))
            .unwrap_or_else(|| crate::scene::tile_companion(&path, ".terrain"));
        let terrain = omsi_map::Terrain::load(&terrain_path).ok()?;
        Some(terrain.sample((camera.position.x - t.x as f64 * size) as f32,
            (camera.position.y - t.y as f64 * size) as f32) as f64)
    }).unwrap_or(0.0);
    let height = if remembered { camera.position.z } else { ground + 35.0 };
    camera.position.z = omsi_launcher_lib::editor_views::start_height(height, ground);
}
