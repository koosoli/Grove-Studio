use crate::tree::{self, Settings};
use serde_json::{json, Value};
use std::{
    io::{self, BufRead, Write},
    path::{Path, PathBuf},
};

pub const HELP: &str = "Grove Studio

Editor: grove-studio [--preset NAME] [--config FILE] [--forest SPECIES]
Mesh export: grove-studio --headless --preset solstice-oak --out exports/oak.p3d --lod all
Mesh inspection: grove-studio --headless --inspect --preset solstice-birch
Game LOD ladder (near/mid/far/very far, P3D): grove-studio --headless --preset solstice-cwa-buk
  --lod game --out exports/beech.p3d [--prefix PBO_PREFIX] [--forest-proxy] (also with --inspect)
Species file: --species-file species/forest/forest-beech.species --preset solstice-cwa-buk
  grows the preset from that text (forest-grown heights up to 40 m).
PNG preview: grove-studio --headless --preview --out exports/forest.png --forest solstice-oak,solstice-fir

Presets: solstice-oak, solstice-birch, solstice-fir, solstice-bush, solstice-poseidon-oak.
Mesh options: --preset NAME, --config FILE, --seed N, --height METERS, --density .25..1.8,
--season summer|autumn|spring|winter, --parallax-depth 0..0.08,
--format gltf|obj|p3d, --lod 0|1|2|all, --count 1..100, --out FILE.
Preview options: --forest SPECIES, --preview-view lit|clusters|gi|voxels|reflections,
--distance METERS, --wind-time SECONDS, --lod auto|0|1|2,
--time-of-day HOURS, --sun-azimuth DEGREES, --skybox clear|fair|broken|overcast|storm|cirrus.
MCP: grove-studio --mcp [--output-root DIRECTORY]
Validation: --self-test results.json; --screenshot image.png
";

pub fn option(args: &[String], key: &str) -> Option<String> {
    args.windows(2).find(|a| a[0] == key).map(|a| a[1].clone())
}
pub fn dispatch(args: &[String]) -> Option<Result<(), String>> {
    if args.iter().any(|a| a == "--version") {
        println!("Grove Studio {}", env!("CARGO_PKG_VERSION"));
        return Some(Ok(()));
    }
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("{HELP}");
        return Some(Ok(()));
    }
    if args.iter().any(|a| a == "--mcp") {
        return Some(mcp_root(args).and_then(mcp));
    }
    if !args.iter().any(|a| a == "--headless") {
        return None;
    }
    Some(headless(args))
}
fn mcp_root(args: &[String]) -> Result<PathBuf, String> {
    let mut root = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--mcp" => i += 1,
            "--output-root" => {
                let value = args
                    .get(i + 1)
                    .filter(|s| !s.is_empty() && !s.starts_with("--"))
                    .ok_or("Missing value for --output-root")?;
                if root.is_some() {
                    return Err("Specify --output-root only once".into());
                }
                root = Some(PathBuf::from(value));
                i += 2;
            }
            other => return Err(format!("Unknown MCP option: {other}")),
        }
    }
    Ok(root.unwrap_or_else(|| PathBuf::from("exports")))
}
fn headless(args: &[String]) -> Result<(), String> {
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--headless" | "--inspect" | "--preview" | "--forest-proxy" => i += 1,
            "--preset" | "--seed" | "--height" | "--density" | "--format" | "--lod" | "--count"
            | "--prefix" | "--species-file"
            | "--config" | "--out" | "--season" | "--parallax-depth" | "--forest"
            | "--preview-view" | "--distance" | "--wind-time" | "--time-of-day" | "--forest-count"
            | "--sun-azimuth" | "--skybox" => {
                if i + 1 == args.len() || args[i + 1].starts_with("--") {
                    return Err(format!("Missing value for {}", args[i]));
                }
                i += 2
            }
            other => return Err(format!("Unknown option: {other}")),
        }
    }
    if let Some(file) = option(args, "--species-file") {
        // The species text replaces the preset's own definition; the preset names its bark.
        let id = option(args, "--preset").ok_or("--species-file needs --preset (its base species)")?;
        let text = std::fs::read_to_string(&file).map_err(|e| format!("{file}: {e}"))?;
        crate::reference::override_species(&id.to_lowercase(), text)?;
    }
    let mut s = if let Some(path) = option(args, "--config") {
        tree::load_project(Path::new(&path))?
    } else {
        Settings::preset(&option(args, "--preset").unwrap_or_else(|| "solstice-oak".into()))?
    };
    if let Some(v) = option(args, "--seed") {
        s.seed = v.parse().map_err(|_| "Invalid seed")?
    }
    if let Some(v) = option(args, "--height") {
        s.height = v.parse().map_err(|_| "Invalid height")?
    }
    if let Some(v) = option(args, "--density") {
        s.density = v.parse().map_err(|_| "Invalid density")?
    }
    if let Some(v) = option(args, "--season") {
        s.season = match v.as_str() {
            "summer" | "0" => 0,
            "autumn" | "1" => 1,
            "spring" | "2" => 2,
            "winter" | "3" => 3,
            _ => return Err("Season must be summer, autumn, spring or winter".into()),
        };
    }
    if let Some(v) = option(args, "--parallax-depth") {
        s.parallax_depth = v.parse().map_err(|_| "Invalid parallax depth")?;
    }
    s.validate()?;
    if args.iter().any(|a| a == "--preview") {
        if ["--inspect", "--format", "--count"]
            .iter()
            .any(|flag| args.iter().any(|arg| arg == flag))
        {
            return Err("Preview cannot be combined with inspect, format or count".into());
        }
        let path =
            PathBuf::from(option(args, "--out").unwrap_or_else(|| "exports/preview.png".into()));
        let mut preview = json!({});
        for (flag, key) in [
            ("--forest", "forest"),
            ("--preview-view", "view"),
            ("--lod", "lod"),
            ("--skybox", "skybox"),
        ] {
            if let Some(value) = option(args, flag) {
                preview[key] = json!(value);
            }
        }
        for (flag, key) in [
            ("--distance", "distance"),
            ("--forest-count", "treeCount"),
            ("--wind-time", "windTime"),
            ("--time-of-day", "timeOfDay"),
            ("--sun-azimuth", "sunAzimuth"),
        ] {
            if let Some(value) = option(args, flag) {
                let value: f64 = value
                    .parse()
                    .map_err(|_| format!("Invalid value for {flag}"))?;
                preview[key] = json!(value);
            }
        }
        println!("{}", capture_preview(s, &preview, &path)?);
        return Ok(());
    }
    if [
        "--forest",
        "--forest-count",
        "--skybox",
        "--preview-view",
        "--distance",
        "--wind-time",
        "--time-of-day",
        "--sun-azimuth",
    ]
    .iter()
    .any(|flag| args.iter().any(|arg| arg == flag))
    {
        return Err("Forest, view, distance and wind time options require --preview".into());
    }
    let lod = option(args, "--lod").unwrap_or_else(|| "all".into());
    let game = lod == "game";
    let forest_proxy = args.iter().any(|a| a == "--forest-proxy");
    if (forest_proxy || option(args, "--prefix").is_some()) && !game {
        return Err("--prefix and --forest-proxy need --lod game".into());
    }
    if game {
        if args.iter().any(|a| a == "--inspect") {
            let ladder = crate::gamelod::ladder(&s);
            println!("{}", json!({"settings":s,"levels":crate::gamelod::stats(&ladder)}));
            return Ok(());
        }
        let out = PathBuf::from(option(args, "--out").unwrap_or_else(|| "exports/tree.p3d".into()));
        let format = option(args, "--format").unwrap_or_else(|| "p3d".into());
        if format != "p3d" || out.extension().and_then(|x| x.to_str()) != Some("p3d") {
            return Err("--lod game writes a Poseidon .p3d".into());
        }
        let count: u32 = option(args, "--count")
            .unwrap_or_else(|| "1".into())
            .parse()
            .map_err(|_| "Invalid count")?;
        let options = crate::poseidon::GameOptions {
            prefix: option(args, "--prefix"),
            forest_proxy,
        };
        let mut files = vec![];
        let seed = s.seed;
        for n in 0..count {
            s.seed = seed.wrapping_add(n);
            let path = output_name(&out, 0, false, (count > 1).then_some(n), "p3d");
            files.push(crate::poseidon::export_game(&s, &path, &options)?);
        }
        println!("{}", json!({"ok":true,"files":files}));
        return Ok(());
    }
    let lods = parse_lods(&lod)?;
    if args.iter().any(|a| a == "--inspect") {
        println!(
            "{}",
            json!({"settings":s,"lods":lods.iter().map(|n|tree::generate(&s,*n).stats()).collect::<Vec<_>>()})
        );
        return Ok(());
    }
    let out = PathBuf::from(option(args, "--out").unwrap_or_else(|| "exports/tree.gltf".into()));
    let format = option(args, "--format").unwrap_or_else(|| {
        out.extension()
            .and_then(|x| x.to_str())
            .unwrap_or("gltf")
            .into()
    });
    let count: u32 = option(args, "--count")
        .unwrap_or_else(|| "1".into())
        .parse()
        .map_err(|_| "Invalid count")?;
    if !(1..=100).contains(&count) {
        return Err("Count must be 1..100".into());
    }
    let mut files = vec![];
    let seed = s.seed;
    for n in 0..count {
        s.seed = seed.wrapping_add(n);
        files.extend(export_lods(
            &s,
            &lods,
            &format,
            &out,
            (count > 1).then_some(n),
            None,
        )?);
    }
    println!("{}", json!({"ok":true,"files":files}));
    Ok(())
}
pub fn export_lods(
    settings: &Settings,
    lods: &[u32],
    format: &str,
    base: &Path,
    index: Option<u32>,
    root: Option<&Path>,
) -> Result<Vec<Value>, String> {
    if !["gltf", "obj", "p3d"].contains(&format) {
        return Err("Format must be gltf, obj or p3d".into());
    }
    if format == "p3d" {
        let path = output_name(base, 0, false, index, format);
        check_outputs(
            root,
            std::iter::once(path.clone()).chain(crate::poseidon::sidecars(&path)),
        )?;
        return Ok(vec![crate::poseidon::export(settings, lods, &path)?]);
    }
    let mut files = Vec::new();
    for &lod in lods {
        let path = output_name(base, lod, lods.len() > 1, index, format);
        let extensions: &[_] = if format == "gltf" {
            &["bin"]
        } else {
            &["mtl", "bark.png", "bark-height.png", "bark-normal.png"]
        };
        check_outputs(
            root,
            std::iter::once(path.clone())
                .chain(extensions.iter().map(|ext| path.with_extension(ext))),
        )?;
        files.push(tree::export(settings, lod, format, &path)?);
    }
    Ok(files)
}

fn check_outputs(
    root: Option<&Path>,
    paths: impl IntoIterator<Item = PathBuf>,
) -> Result<(), String> {
    if let Some(root) = root {
        for path in paths {
            if std::fs::symlink_metadata(&path).is_ok()
                && !path
                    .canonicalize()
                    .map_err(|e| e.to_string())?
                    .starts_with(root)
            {
                return Err("Output file escapes output root".into());
            }
        }
    }
    Ok(())
}

pub fn output_name(
    base: &Path,
    lod: u32,
    multi: bool,
    index: Option<u32>,
    format: &str,
) -> PathBuf {
    let mut name = base
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    if name.is_empty() {
        name = "tree".into()
    }
    if let Some(n) = index {
        name.push_str(&format!("_{n:03}"))
    }
    if multi {
        name.push_str(&format!("_lod{lod}"))
    }
    base.with_file_name(format!("{name}.{format}"))
}
pub fn parse_lods(s: &str) -> Result<Vec<u32>, String> {
    match s {
        "all" => Ok(vec![0, 1, 2]),
        "0" => Ok(vec![0]),
        "1" => Ok(vec![1]),
        "2" => Ok(vec![2]),
        _ => Err("LOD must be 0, 1, 2 or all".into()),
    }
}
fn schema() -> Value {
    json!({"type":"object","additionalProperties":false,"properties":{"species":{"type":"string","enum":crate::preview::SPECIES.as_slice()},"seed":{"type":"integer","minimum":0,"maximum":4294967295u64},"height":{"type":"number","minimum":0.25,"maximum":16},"spread":{"type":"number","minimum":0.4,"maximum":1.6},"branches":{"type":"integer","minimum":5,"maximum":20},"density":{"type":"number","minimum":0.25,"maximum":1.8},"leaf_size":{"type":"number","minimum":0.025,"maximum":0.4},"trunk_radius":{"type":"number","minimum":0.05,"maximum":0.45},"season":{"type":"integer","minimum":0,"maximum":3},"parallax_depth":{"type":"number","minimum":0,"maximum":0.08}}})
}
pub fn tools_list() -> Value {
    json!({"tools":[
        {"name":"tree_presets","description":"List the available tree species and their default generation settings.","inputSchema":{"type":"object","properties":{},"additionalProperties":false}},
        {"name":"tree_inspect","description":"Generate tree statistics for all detail levels without opening a window or writing files.","inputSchema":{"type":"object","properties":{"settings":schema()},"additionalProperties":false}},
        {"name":"tree_generate","description":"Generate a deterministic tree and export glTF, OBJ or Poseidon P3D beneath the output root.","inputSchema":{"type":"object","properties":{"settings":schema(),"output":{"type":"string","description":"Relative output name, e.g. oak.p3d"},"format":{"type":"string","enum":["gltf","obj","p3d"]},"lod":{"type":"string","enum":["all","0","1","2"]}},"required":["output"],"additionalProperties":false}},
        {"name":"tree_preview","description":"Render a Solstice PNG and return measured visible triangle and cluster counts. Requires a compatible GPU, with no window.","inputSchema":{"type":"object","properties":{"settings":schema(),"output":{"type":"string","description":"Relative PNG filename"},"forest":{"type":"string","description":"Comma-separated species, e.g. solstice-oak,solstice-fir"},"lod":{"type":"string","enum":["auto","0","1","2"]},"view":{"type":"string","enum":["lit","clusters","triangles","gi","reflections","voxels","albedo","normals"]},"distance":{"type":"number","minimum":0.15,"maximum":500},"sky":{"type":"boolean"},"skybox":{"type":"string","enum":["clear","fair","broken","overcast","storm","cirrus"]},"windDirection":{"type":"number","minimum":0,"maximum":360},"treeCount":{"type":"integer","minimum":1,"maximum":1000},"ratios":{"type":"array","minItems":crate::preview::SPECIES.len(),"maxItems":crate::preview::SPECIES.len(),"items":{"type":"integer","minimum":0,"maximum":100}},"windTime":{"type":"number","minimum":0,"maximum":86400},"timeOfDay":{"type":"number","minimum":0,"maximum":24},"sunAzimuth":{"type":"number","minimum":0,"maximum":360}},"required":["output"],"additionalProperties":false}}
    ]})
}
fn settings_from(v: &Value) -> Result<Settings, String> {
    let settings = v.get("settings").cloned().unwrap_or(json!({}));
    let obj = settings.as_object().ok_or("settings must be an object")?;
    let preset = obj
        .get("species")
        .and_then(Value::as_str)
        .unwrap_or("solstice-oak");
    let mut base = serde_json::to_value(Settings::preset(preset)?).unwrap();
    for (k, v) in obj {
        base[k] = v.clone()
    }
    let s: Settings = serde_json::from_value(base).map_err(|e| e.to_string())?;
    s.validate()?;
    Ok(s)
}
pub fn call_tool(name: &str, args: &Value, root: &Path) -> Result<Value, String> {
    let obj = args.as_object().ok_or("arguments must be an object")?;
    let allowed: &[&str] = match name {
        "tree_presets" => &[],
        "tree_inspect" => &["settings"],
        "tree_generate" => &["settings", "output", "format", "lod"],
            "tree_preview" => &[
            "settings",
            "output",
            "forest",
            "lod",
            "view",
            "distance",
            "windTime",
            "windDirection",
            "sky",
            "skybox",
            "ratios",
            "treeCount",
            "timeOfDay",
            "sunAzimuth",
        ],
        _ => return Err("Unknown tool".into()),
    };
    for key in obj.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(format!("Unknown argument: {key}"));
        }
    }
    if name == "tree_presets" {
        return Ok(
            json!({"presets":crate::preview::SPECIES.iter().map(|name| Settings::preset(name).expect("known preset")).collect::<Vec<_>>()}),
        );
    }
    let s = settings_from(args)?;
    if name == "tree_inspect" {
        return Ok(
            json!({"settings":s,"lods":(0..3).map(|n|tree::generate(&s,n).stats()).collect::<Vec<_>>()}),
        );
    }
    let file = args
        .get("output")
        .and_then(Value::as_str)
        .ok_or("output must be a relative filename")?;
    let rel = Path::new(file);
    if rel.as_os_str().is_empty()
        || rel
            .components()
            .any(|c| !matches!(c, std::path::Component::Normal(_)))
    {
        return Err("Output must be a relative name beneath the output root, without ..".into());
    }
    // Resolve existing ancestors so a symlink cannot escape the server's output root.
    std::fs::create_dir_all(root).map_err(|e| e.to_string())?;
    let root = root.canonicalize().map_err(|e| e.to_string())?;
    let base = root.join(rel);
    let parent = base.parent().ok_or("Invalid path")?;
    let mut ancestor = parent;
    while !ancestor.exists() {
        ancestor = ancestor.parent().ok_or("Invalid path")?
    }
    if !ancestor
        .canonicalize()
        .map_err(|e| e.to_string())?
        .starts_with(&root)
    {
        return Err("Output directory escapes output root".into());
    }
    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    if name == "tree_preview" {
        check_outputs(Some(&root), [base.clone()])?;
        return capture_preview(s, args, &base);
    }
    let format = match args.get("format") {
        Some(v) => v.as_str().ok_or("format must be a string")?,
        None => rel.extension().and_then(|s| s.to_str()).unwrap_or("gltf"),
    };
    let lod = match args.get("lod") {
        Some(v) => v.as_str().ok_or("lod must be a string")?,
        None => "all",
    };
    let lods = parse_lods(lod)?;
    let files = export_lods(&s, &lods, format, &base, None, Some(&root))?;
    Ok(json!({"ok":true,"files":files}))
}

fn capture_preview(s: Settings, args: &Value, path: &Path) -> Result<Value, String> {
    use crate::preview::{self, Request, SceneKey};
    if path.extension().and_then(|ext| ext.to_str()) != Some("png") {
        return Err("Preview output must have the .png extension".into());
    }
    let string = |key| {
        args.get(key)
            .map(|v| v.as_str().ok_or_else(|| format!("{key} must be a string")))
            .transpose()
    };
    let forest = string("forest")?
        .map(preview::forest_mask)
        .transpose()?
        .unwrap_or(0);
    let count = args.get("treeCount").map_or(Ok(200), |value| {
        value
            .as_f64()
            .filter(|n| n.fract() == 0. && (1. ..=1000.).contains(n))
            .map(|n| n as u32)
            .ok_or("treeCount must be an integer from 1 to 1000")
    })?;
    let view = string("view")?.unwrap_or("lit");
    let debug = solstice_render::DebugView::from_name(view).ok_or("Unknown debug view")?;
    let lod = match string("lod")?.unwrap_or("auto") {
        "auto" => None,
        "0" => Some(0),
        "1" => Some(1),
        "2" => Some(2),
        _ => return Err("Preview LOD must be auto, 0, 1 or 2".into()),
    };
    let distance = args
        .get("distance")
        .map(|value| {
            value
                .as_f64()
                .filter(|v| (0.15..=500.).contains(v))
                .map(|v| v as f32)
                .ok_or("Preview distance must be 0.15..500 meters")
        })
        .transpose()?;
    let wind_time = args
        .get("windTime")
        .map(|value| {
            value
                .as_f64()
                .filter(|v| (0. ..=86400.).contains(v))
                .map(|v| v as f32)
                .ok_or("Preview wind time must be 0..86400 seconds")
        })
        .transpose()?;
    let number = |key, max, default| {
        args.get(key).map_or(Ok(default), |value: &Value| {
            value
                .as_f64()
                .filter(|v| (0. ..=max).contains(v))
                .map(|v| v as f32)
                .ok_or_else(|| format!("{key} must be 0..{max}"))
        })
    };
    let defaults = preview::Lighting::default();
    let lighting = preview::Lighting {
        hours: number("timeOfDay", 24., defaults.hours)?,
        azimuth: number("sunAzimuth", 360., defaults.azimuth)?,
    };
    // Ratios are percentage shares per species; the mix is normalised, so they need not add to 100.
    let mut ratios = [1; preview::SPECIES.len()];
    if let Some(value) = args.get("ratios") {
        let values = value
            .as_array()
            .filter(|values| values.len() == preview::SPECIES.len())
            .ok_or("ratios must contain one integer per species")?;
        for (target, value) in ratios.iter_mut().zip(values) {
            *target = value
                .as_u64()
                .filter(|n| *n <= 100)
                .ok_or("ratio must be an integer from 0 to 100")? as u32;
        }
        if forest != 0
            && !(0..preview::SPECIES.len()).any(|i| forest & (1 << i) != 0 && ratios[i] > 0)
        {
            return Err("At least one selected species must have a positive ratio".into());
        }
    }
    let sky = args
        .get("sky")
        .map(|v| v.as_bool().ok_or("sky must be boolean"))
        .transpose()?
        .unwrap_or(true);
    let skybox_name = string("skybox")?.unwrap_or("fair");
    let skybox = preview::skybox(skybox_name)?;
    let frame = preview::capture(
        Request {
            camera: preview::camera_for(&s, forest, count, distance),
            scene: SceneKey {
                settings: s,
                lod,
                forest,
                count,
                ratios,
                grid: forest == 0,
                bark: true,
                leaves: true,
            },
            debug,
            wind_time,
            wind_direction: number("windDirection", 360., 68.)?,
            sky,
            skybox,
            lighting,
        },
        path,
    )?;
    Ok(
        json!({"path":path,"view":view,"skybox":skybox_name,"timeOfDay":lighting.hours,"sunAzimuth":lighting.azimuth,"clusters":frame.clusters,"drawnTriangles":frame.triangles}),
    )
}
fn rpc(req: Value, root: &Path) -> Option<Value> {
    if !req.is_object()
        || req.get("jsonrpc").and_then(Value::as_str) != Some("2.0")
        || req.get("method").and_then(Value::as_str).is_none()
    {
        return Some(
            json!({"jsonrpc":"2.0","id":null,"error":{"code":-32600,"message":"Invalid Request"}}),
        );
    }
    let id = req.get("id").cloned();
    let method = req.get("method").and_then(Value::as_str).unwrap_or("");
    if id.is_none() {
        return None;
    }
    let id = id.unwrap();
    let result = match method {
        "initialize" => {
            let requested = req
                .pointer("/params/protocolVersion")
                .and_then(Value::as_str)
                .unwrap_or("2025-03-26");
            let version =
                if ["2024-11-05", "2025-03-26", "2025-06-18", "2025-11-25"].contains(&requested) {
                    requested
                } else {
                    "2025-03-26"
                };
            json!({"protocolVersion":version,"capabilities":{"tools":{"listChanged":false}},"serverInfo":{"name":"grove-studio","version":"0.1.0"},"instructions":"Use tree_presets, then tree_inspect or tree_generate. Generation is deterministic; units are meters; the Y axis is up."})
        }
        "ping" => json!({}),
        "tools/list" => tools_list(),
        "tools/call" => {
            let name = req
                .pointer("/params/name")
                .and_then(Value::as_str)
                .unwrap_or("");
            let args = req
                .pointer("/params/arguments")
                .cloned()
                .unwrap_or(json!({}));
            match call_tool(name, &args, root) {
                Ok(v) => json!({"content":[{"type":"text","text":v.to_string()}],"isError":false}),
                Err(e) => json!({"content":[{"type":"text","text":e}],"isError":true}),
            }
        }
        _ => {
            return Some(
                json!({"jsonrpc":"2.0","id":id,"error":{"code":-32601,"message":"Method not found"}}),
            )
        }
    };
    Some(json!({"jsonrpc":"2.0","id":id,"result":result}))
}
fn mcp(root: PathBuf) -> Result<(), String> {
    let stdin = io::stdin();
    let mut stdout = io::stdout().lock();
    for line in stdin.lock().lines() {
        let line = line.map_err(|e| e.to_string())?;
        if line.trim().is_empty() {
            continue;
        }
        let response = match serde_json::from_str::<Value>(&line) {
            Ok(req) => rpc(req, &root),
            Err(_) => Some(
                json!({"jsonrpc":"2.0","id":null,"error":{"code":-32700,"message":"Parse error"}}),
            ),
        };
        if let Some(v) = response {
            writeln!(stdout, "{v}").map_err(|e| e.to_string())?;
            stdout.flush().map_err(|e| e.to_string())?
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mcp_rejects_invalid_calls() {
        assert!(call_tool(
            "tree_generate",
            &json!({"output":"../escape.gltf"}),
            Path::new("exports")
        )
        .is_err());
        assert!(call_tool(
            "tree_inspect",
            &json!({"settings":{"height":999}}),
            Path::new("exports")
        )
        .is_err());
        assert_eq!(tools_list()["tools"].as_array().unwrap().len(), 4);
        assert!(rpc(
            json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
            Path::new("exports")
        )
        .is_none());
    }
}
