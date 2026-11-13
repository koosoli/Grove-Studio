use crate::tree::{self, Settings};
use serde_json::{json, Value};
use std::{
    io::{self, BufRead, Write},
    path::{Path, PathBuf},
};

pub const HELP: &str = "Grove Studio

Grass: grove-studio --grass meadow [--headless --inspect]
Grass mesh: --headless --grass meadow --out exports/grass.gltf --lod all
Engine textures: --headless --grass meadow --format engine --out exports/grass-pack
Grass controls: --size .25..4 --density .05..1.5 --height .25..2 --width .5..3 --lean 0..1.5 --dry 0..1 --no-weeds
Editor: grove-studio [--preset NAME] [--config FILE] [--forest SPECIES]
Mesh export: grove-studio --headless --preset solstice-oak --out exports/oak.p3d --lod all
Mesh inspection: grove-studio --headless --inspect --preset solstice-birch
Budget report: grove-studio --budget-report [--preset SPECIES] [--no-trim-cards]
Game LOD ladder (near/mid/far/very far, P3D): grove-studio --headless --preset solstice-cwa-buk
  --lod game --out exports/beech.p3d [--prefix PBO_PREFIX] [--forest-proxy] (also with --inspect)
  [--no-trim-cards] [--budgets near,mid,far,veryfar] [--opaque-leaves [--budgets near,veryfar]]: opaque leaf pieces
  in one near level for an engine that simplifies per cluster, plus the very far spray level
Species file: --species-file species/forest/forest-beech.species --preset solstice-cwa-buk
  grows the preset from that text (forest-grown heights up to 40 m).
PNG preview: grove-studio --headless --preview --out exports/forest.png --forest solstice-oak,solstice-fir

Presets: solstice-oak, solstice-birch, solstice-fir, solstice-bush, solstice-poseidon-oak.
Material PNGs: --bark-color FILE --bark-height FILE --bark-normal FILE --leaf-atlas FILE
Mesh options: --preset NAME, --config FILE, --seed N, --height METERS, --maturity .15..1.2 (bush .4 minimum), --density .25..1.8,
--randomize all|shape|foliage OR --random-field NAME, with --random-seed N,
--season summer|autumn|spring|winter, --parallax-depth 0..0.08, --bark-weathering 0..1,
--format gltf|obj|p3d, --lod 0|1|2|all, --count 1..100, --out FILE.
Preview options: --forest SPECIES, --preview-view lit|clusters|gi|voxels|reflections,
--distance METERS, --wind-time SECONDS, --lod auto|0|1|2|3 (game cards),
--no-forest-floor, --even-mix, --structure (hide foliage),
--time-of-day HOURS, --sun-azimuth DEGREES, --skybox clear|fair|broken|overcast|storm|cirrus,
--cloud-cover 0..1, --leaf-roughness 0.05..1, --leaf-transmission 0..1,
--wind-direction 0..360, --wind-strength 0..2, --wind-gust 0..1, --branch-sway 0..2, --branch-springiness 0..1, --leaf-flutter 0..2, --no-leaf-specular,
--no-trim-cards, --original-leaves.
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
    if args.iter().any(|a|a=="--grass") && args.iter().any(|a|a=="--headless") {return Some(crate::grass::headless(args));}
    if !args.iter().any(|a| a == "--headless" || a == "--budget-report") {
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
            "--headless" | "--budget-report" | "--inspect" | "--preview" | "--forest-proxy" | "--opaque-leaves" | "--no-trim-cards" | "--original-leaves" | "--no-leaf-specular" | "--no-forest-floor" | "--even-mix" | "--structure" => i += 1,
            "--randomize" | "--random-field" | "--random-seed" | "--bark-weathering" | "--bark-color" | "--bark-height" | "--bark-normal" | "--leaf-atlas" | "--preset" | "--seed" | "--height" | "--maturity" | "--density" | "--format" | "--lod" | "--count"
            | "--prefix" | "--species-file" | "--budgets"
            | "--config" | "--out" | "--season" | "--parallax-depth" | "--forest"
            | "--preview-view" | "--distance" | "--wind-time" | "--time-of-day" | "--forest-count"
            | "--sun-azimuth" | "--skybox" | "--cloud-cover" | "--leaf-roughness" | "--leaf-transmission" | "--wind-direction" | "--wind-strength" | "--wind-gust" | "--branch-sway" | "--branch-springiness" | "--leaf-flutter" => {
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
    let scope = option(args, "--randomize");
    let field = option(args, "--random-field");
    if scope.is_some() && field.is_some() { return Err("Use either --randomize or --random-field".into()); }
    if scope.is_some() || field.is_some() {
        let seed = option(args, "--random-seed").map(|v| v.parse().map_err(|_| "Invalid random seed")).transpose()?.unwrap_or(s.seed);
        if let Some(field) = field { s.randomize_field(&field, seed)?; } else { s.randomize(scope.as_deref().unwrap(), seed)?; }
    } else if option(args, "--random-seed").is_some() { return Err("--random-seed requires --randomize or --random-field".into()); }
    if let Some(v) = option(args, "--seed") {
        s.seed = v.parse().map_err(|_| "Invalid seed")?
    }
    if let Some(v) = option(args, "--height") {
        s.height = v.parse().map_err(|_| "Invalid height")?
    }
    if let Some(v) = option(args, "--maturity") {
        s.maturity = v.parse().map_err(|_| "Invalid maturity")?
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
    if let Some(v) = option(args, "--bark-weathering") { s.bark_weathering = v.parse().map_err(|_| "Invalid bark weathering")?; }
    if let Some(v) = option(args, "--bark-color") { s.bark_color_path = Some(v); }
    if let Some(v) = option(args, "--bark-height") { s.bark_height_path = Some(v); }
    if let Some(v) = option(args, "--bark-normal") { s.bark_normal_path = Some(v); }
    if let Some(v) = option(args, "--leaf-atlas") { s.leaf_atlas_path = Some(v); }
    s.validate()?;
    if args.iter().any(|a| a == "--budget-report") {
        if args.iter().any(|a| ["--preview", "--inspect", "--out", "--budgets", "--opaque-leaves"].contains(&a.as_str())) {
            return Err("--budget-report cannot be combined with preview, inspect, export, custom budgets or opaque-leaves".into());
        }
        let names: Vec<String> = if option(args, "--preset").is_some() || option(args, "--config").is_some() {
            vec![s.species.clone()]
        } else {
            ["beech", "spruce", "pine", "oak", "birch", "lime", "willow"].iter().map(|n| format!("solstice-forest-{n}")).collect()
        };
        let mut reports = Vec::new();
        for name in names {
            let settings = if name == s.species { s.clone() } else { Settings::preset(&name)? };
            let mut ladder = crate::gamelod::ladder(&settings);
            if !args.iter().any(|a| a == "--no-trim-cards") { crate::gamelod::trim_cards(&mut ladder); }
            let proxy = crate::gamelod::proxy_ladder(&settings);
            reports.push(json!({"species":name, "settings":settings, "levels":crate::gamelod::stats(&ladder), "shadowAndOcclusionProxy":crate::gamelod::stats(&proxy)}));
        }
        println!("{}", json!({"type":"grove-budget-report", "geometryBytes":"in-memory V records plus u32 indices, not GPU allocation or textures", "species":reports}));
        return Ok(());
    }
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
        if args.iter().any(|a| a == "--no-trim-cards") { preview["trimCards"] = json!(false); }
        if args.iter().any(|a| a == "--original-leaves") { preview["gameCards"] = json!(false); }
        if args.iter().any(|a| a == "--no-leaf-specular") { preview["leafSpecular"] = json!(false); }
        if args.iter().any(|a| a == "--no-forest-floor") { preview["forestFloor"] = json!(false); }
        if args.iter().any(|a| a == "--even-mix") { preview["randomMix"] = json!(false); }
        if args.iter().any(|a| a == "--structure") { preview["structure"] = json!(true); }
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
            ("--cloud-cover", "cloudCover"),
            ("--leaf-roughness", "leafRoughness"),
            ("--leaf-transmission", "leafTransmission"),
            ("--wind-direction", "windDirection"),
            ("--wind-strength", "windStrength"),
            ("--wind-gust", "windGust"),
            ("--branch-sway", "branchSway"),
            ("--branch-springiness", "branchSpringiness"),
            ("--leaf-flutter", "leafFlutter"),
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
        "--cloud-cover",
        "--leaf-roughness",
        "--leaf-transmission",
        "--wind-direction",
        "--wind-strength",
        "--wind-gust",
        "--branch-sway",
        "--branch-springiness",
        "--leaf-flutter",
        "--no-leaf-specular",
        "--no-forest-floor",
        "--even-mix",
        "--structure",
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
    if !game && args.iter().any(|a| a == "--no-trim-cards") {
        return Err("--no-trim-cards needs --lod game".into());
    }
    let forest_proxy = args.iter().any(|a| a == "--forest-proxy");
    let opaque_leaves = args.iter().any(|a| a == "--opaque-leaves");
    if opaque_leaves && !game {
        return Err("--opaque-leaves needs --lod game".into());
    }
    if (forest_proxy || option(args, "--prefix").is_some()) && !game {
        return Err("--prefix and --forest-proxy need --lod game".into());
    }
    let budgets = match option(args, "--budgets") {
        Some(v) if game => Some(
            v.split(',')
                .map(|n| n.trim().parse::<usize>().map_err(|_| "Invalid --budgets"))
                .collect::<Result<Vec<_>, _>>()?,
        ),
        Some(_) => return Err("--budgets needs --lod game".into()),
        None => None,
    };
    if game {
        if args.iter().any(|a| a == "--inspect") {
            let mut ladder = if opaque_leaves {
                crate::gamelod::ladder_with(&s, &crate::gamelod::opaque_rungs(budgets.as_deref())?)
            } else {
                match &budgets {
                    Some(b) => crate::gamelod::ladder_with(&s, &crate::gamelod::rungs_with_budgets(b)?),
                    None => crate::gamelod::ladder(&s),
                }
            };
            if !args.iter().any(|a| a == "--no-trim-cards") { crate::gamelod::trim_cards(&mut ladder); }
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
            budgets,
            opaque_leaves,
            trim_cards: !args.iter().any(|a| a == "--no-trim-cards"),
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
            &["mtl", "bark.png", "bark-height.png", "bark-normal.png", "leaves.png"]
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
    json!({"type":"object","additionalProperties":false,"properties":{"species":{"type":"string","enum":crate::preview::SPECIES.as_slice()},"seed":{"type":"integer","minimum":0,"maximum":4294967295u64},"height":{"type":"number","minimum":0.25,"maximum":40},"maturity":{"type":"number","minimum":0.15,"maximum":1.2,"default":1},"spread":{"type":"number","minimum":0.4,"maximum":1.6},"branches":{"type":"integer","minimum":5,"maximum":20},"density":{"type":"number","minimum":0.25,"maximum":1.8},"leaf_size":{"type":"number","minimum":0.025,"maximum":0.4},"trunk_radius":{"type":"number","minimum":0.05,"maximum":1.0},"trunk_straightness":{"type":"number","minimum":0,"maximum":1},"branch_angle":{"type":"number","minimum":0.5,"maximum":1.5},"branch_droop":{"type":"number","minimum":0,"maximum":2},"season":{"type":"integer","minimum":0,"maximum":3},"parallax_depth":{"type":"number","minimum":0,"maximum":0.08},"bark_weathering":{"type":"number","minimum":0,"maximum":1},"bark_color_path":{"type":["string","null"]},"bark_height_path":{"type":["string","null"]},"bark_normal_path":{"type":["string","null"]},"leaf_atlas_path":{"type":["string","null"]}}})
}
fn grass_schema()->Value { json!({"type":"object","additionalProperties":false,"properties":{"preset":{"type":"string","enum":crate::grass::PRESETS},"style":{"type":"string","enum":["solstice","poseidon"]},"seed":{"type":"integer","minimum":0,"maximum":4294967295u64},"size":{"type":"number","minimum":0.25,"maximum":4},"density":{"type":"number","minimum":0.05,"maximum":1.5},"height":{"type":"number","minimum":0.25,"maximum":2},"width":{"type":"number","minimum":0.5,"maximum":3},"lean":{"type":"number","minimum":0,"maximum":1.5},"dry":{"type":"number","minimum":0,"maximum":1},"patchiness":{"type":"number","minimum":0,"maximum":1},"weeds":{"type":"boolean"}}}) }
pub fn tools_list() -> Value {
    json!({"tools":[
        {"name":"grass_presets","description":"List editable procedural grass presets.","inputSchema":{"type":"object","additionalProperties":false}},
        {"name":"grass_inspect","description":"Measure the three grass mesh LODs without files.","inputSchema":{"type":"object","properties":{"settings":grass_schema()},"additionalProperties":false}},
        {"name":"grass_generate","description":"Export opaque grass mesh or eight-layer OpenPoseidon texture bundle.","inputSchema":{"type":"object","properties":{"settings":grass_schema(),"output":{"type":"string"},"format":{"type":"string","enum":["gltf","obj","engine"]},"lod":{"type":"integer","minimum":0,"maximum":2}},"required":["output"],"additionalProperties":false}},
        {"name":"grass_preview","description":"Render a grass patch in the Solstice viewer.","inputSchema":{"type":"object","properties":{"settings":grass_schema(),"output":{"type":"string"},"lod":{"type":"integer","minimum":0,"maximum":2},"windTime":{"type":"number","minimum":0,"maximum":86400}},"required":["output"],"additionalProperties":false}},
        {"name":"tree_presets","description":"List tree species and settings; optionally search names and IDs.","inputSchema":{"type":"object","properties":{"query":{"type":"string"}},"additionalProperties":false}},
        {"name":"tree_randomize","description":"Create a reproducible random variation of the current species and selected settings.","inputSchema":{"type":"object","properties":{"settings":schema(),"scope":{"type":"string","enum":["all","shape","foliage"],"default":"all"},"field":{"type":"string","description":"Randomize only this field instead of a scope","enum":["height","maturity","spread","trunk_straightness","branches","branch_angle","branch_droop","trunk_radius","parallax_depth","density","leaf_size","season"]},"seed":{"type":"integer","minimum":0,"maximum":4294967295u64}},"required":["seed"],"additionalProperties":false}},
        {"name":"tree_budget_report","description":"Measure game LOD and shadow-proxy geometry without writing files.","inputSchema":{"type":"object","properties":{"settings":schema(),"trimCards":{"type":"boolean","default":true}},"additionalProperties":false}},
        {"name":"tree_inspect","description":"Generate tree statistics for all detail levels without opening a window or writing files.","inputSchema":{"type":"object","properties":{"settings":schema()},"additionalProperties":false}},
        {"name":"tree_generate","description":"Generate a deterministic tree and export glTF, OBJ or Poseidon P3D beneath the output root.","inputSchema":{"type":"object","properties":{"settings":schema(),"output":{"type":"string","description":"Relative output name, e.g. oak.p3d"},"format":{"type":"string","enum":["gltf","obj","p3d"]},"lod":{"type":"string","enum":["all","0","1","2","game"]},"trimCards":{"type":"boolean","default":true,"description":"Trim alpha card outlines for game LOD P3D exports."}},"required":["output"],"additionalProperties":false}},
        {"name":"tree_preview","description":"Render a Solstice PNG and return measured visible triangle and cluster counts. Requires a compatible GPU, with no window.","inputSchema":{"type":"object","properties":{"settings":schema(),"windStrength":{"type":"number","minimum":0,"maximum":2,"default":1},"branchSpringiness":{"type":"number","minimum":0,"maximum":1,"default":0.5},"branchSway":{"type":"number","minimum":0,"maximum":2,"default":1},"windGust":{"type":"number","minimum":0,"maximum":1,"default":0.2},"leafFlutter":{"type":"number","minimum":0,"maximum":2,"default":1},"leafTransmission":{"type":"number","minimum":0,"maximum":1,"default":0.8},"cloudCover":{"type":"number","minimum":0,"maximum":1},"leafSpecular":{"type":"boolean","default":true},"leafRoughness":{"type":"number","minimum":0.05,"maximum":1,"default":0.55},"structure":{"type":"boolean","default":false},"forestFloor":{"type":"boolean","default":true},"randomMix":{"type":"boolean","default":true},"gameCards":{"type":"boolean","default":true},"trimCards":{"type":"boolean","default":true},"output":{"type":"string","description":"Relative PNG filename"},"forest":{"type":"string","description":"Comma-separated species, e.g. solstice-oak,solstice-fir"},"lod":{"type":"string","enum":["auto","0","1","2","3"]},"view":{"type":"string","enum":["lit","clusters","triangles","gi","reflections","voxels","albedo","normals"]},"distance":{"type":"number","minimum":0.15,"maximum":500},"sky":{"type":"boolean"},"skybox":{"type":"string","enum":["clear","fair","broken","overcast","storm","cirrus"]},"windDirection":{"type":"number","minimum":0,"maximum":360},"treeCount":{"type":"integer","minimum":1,"maximum":1000},"ratios":{"type":"array","minItems":crate::preview::SPECIES.len(),"maxItems":crate::preview::SPECIES.len(),"items":{"type":"integer","minimum":0,"maximum":100}},"windTime":{"type":"number","minimum":0,"maximum":86400},"timeOfDay":{"type":"number","minimum":0,"maximum":24},"sunAzimuth":{"type":"number","minimum":0,"maximum":360}},"required":["output"],"additionalProperties":false}}
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
        "grass_presets"=> &[],
        "grass_inspect"=> &["settings"],
        "grass_generate"=> &["settings","output","format","lod"],
        "grass_preview"=> &["settings","output","lod","windTime"],
        "tree_presets" => &["query"],
        "tree_randomize" => &["settings", "scope", "field", "seed"],
        "tree_inspect" => &["settings"],
        "tree_budget_report" => &["settings", "trimCards"],
        "tree_generate" => &["settings", "output", "format", "lod", "trimCards"],
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
            "gameCards",
            "trimCards",
            "randomMix",
            "forestFloor",
            "structure",
            "cloudCover",
            "leafSpecular",
            "leafRoughness",
            "leafTransmission",
            "windStrength",
            "windGust",
            "branchSway",
            "branchSpringiness",
            "leafFlutter",
        ],
        _ => return Err("Unknown tool".into()),
    };
    for key in obj.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(format!("Unknown argument: {key}"));
        }
    }
    if name.starts_with("grass_") {return grass_tool(name,args,root);}
    if name == "tree_presets" {
        let query = args.get("query").map(|v| v.as_str().ok_or("query must be a string")).transpose()?.unwrap_or("");
        let ids: Vec<&str> = if query.trim().is_empty() { crate::preview::SPECIES.iter().copied().collect() } else { crate::gui::library_matches(query, false).into_iter().map(|v| v.0).collect() };
        return Ok(json!({"presets":ids.iter().map(|name| Settings::preset(name).expect("known preset")).collect::<Vec<_>>() }));
    }
    let mut s = settings_from(args)?;
    if name == "tree_randomize" {
        let seed = args.get("seed").and_then(Value::as_u64).filter(|v| *v <= u32::MAX as u64).ok_or("seed must be an integer from 0 to 4294967295")? as u32;
        let field = args.get("field").map(|v| v.as_str().ok_or("field must be a string")).transpose()?;
        let scope = args.get("scope").map(|v| v.as_str().ok_or("scope must be a string")).transpose()?.unwrap_or("all");
        if let Some(field) = field { s.randomize_field(field, seed)?; } else { s.randomize(scope, seed)?; }
        return Ok(json!({"settings":s,"scope":if field.is_none() { Some(scope) } else { None },"field":field,"randomSeed":seed}));
    }
    if name == "tree_budget_report" {
        let trim = args.get("trimCards").map(|v| v.as_bool().ok_or("trimCards must be a boolean")).transpose()?.unwrap_or(true);
        let mut ladder = crate::gamelod::ladder(&s);
        if trim { crate::gamelod::trim_cards(&mut ladder); }
        let proxy = crate::gamelod::proxy_ladder(&s);
        return Ok(json!({"settings":s,"levels":crate::gamelod::stats(&ladder),"shadowAndOcclusionProxy":crate::gamelod::stats(&proxy),"geometryBytes":"in-memory vertices plus indices; excludes textures and GPU overhead"}));
    }
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
    if lod == "game" {
        if format != "p3d" || rel.extension().and_then(|v| v.to_str()) != Some("p3d") {
            return Err("Game LOD requires a .p3d output and p3d format".into());
        }
        let trim = args.get("trimCards").map(|v| v.as_bool().ok_or("trimCards must be a boolean")).transpose()?.unwrap_or(true);
        check_outputs(Some(&root), std::iter::once(base.clone()).chain(crate::poseidon::game_sidecars(&base)))?;
        let options = crate::poseidon::GameOptions { prefix: None, forest_proxy: false, budgets: None, opaque_leaves: false, trim_cards: trim };
        let file = crate::poseidon::export_game(&s, &base, &options)?;
        return Ok(json!({"ok":true,"files":[file]}));
    }
    if args.get("trimCards").is_some() { return Err("trimCards requires lod=game".into()); }
    let lods = parse_lods(lod)?;
    let files = export_lods(&s, &lods, format, &base, None, Some(&root))?;
    Ok(json!({"ok":true,"files":files}))
}

fn grass_tool(name:&str,args:&Value,root:&Path)->Result<Value,String> {
    if name=="grass_presets" {return Ok(json!({"presets":crate::grass::PRESETS.iter().map(|n|crate::grass::Settings::preset(n).unwrap()).collect::<Vec<_>>()}));}
    let s:crate::grass::Settings=serde_json::from_value(args.get("settings").cloned().unwrap_or(json!({}))).map_err(|e|e.to_string())?;s.validate()?;
    if name=="grass_inspect" {return s.report();}
    let file=args.get("output").and_then(Value::as_str).ok_or("output must be a relative name")?;
    let rel=Path::new(file);
    if rel.as_os_str().is_empty()||rel.components().any(|c|!matches!(c,std::path::Component::Normal(_))){return Err("Output must be beneath output root".into());}
    let lod=args.get("lod").map(|v|v.as_u64().filter(|n|*n<=2).ok_or("Grass LOD must be 0..2")).transpose()?.unwrap_or(0) as u32;
    std::fs::create_dir_all(root).map_err(|e|e.to_string())?;let root=root.canonicalize().map_err(|e|e.to_string())?;let out=root.join(rel);
    let format=args.get("format").map(|v|v.as_str().ok_or("format must be a string")).transpose()?.unwrap_or("gltf");
    let paths:Vec<PathBuf>=if name=="grass_preview"{vec![out.clone()]}else if format=="engine"{
        (0..8).map(|i|out.join(format!("assets/grass/meadow-grass-blade-{i}.png"))).chain([out.join("project.json"),out.join("source.grass"),out.join("grass-bundle.json")]).collect()
    }else{vec![out.clone(),out.with_extension("bin"),out.with_extension("mtl"),out.with_extension("grass.png")]};
    for path in &paths {let mut ancestor=path.as_path();while std::fs::symlink_metadata(ancestor).is_err(){ancestor=ancestor.parent().ok_or("Invalid output path")?;}if !ancestor.canonicalize().map_err(|e|e.to_string())?.starts_with(&root){return Err("Output escapes output root".into());}}
    if name=="grass_preview" {
        let mut request=json!({"grass":s,"lod":lod.to_string()});
        if let Some(time)=args.get("windTime"){request["windTime"]=time.clone();}
        return capture_preview(Settings::default(),&request,&out);
    }
    if format=="engine"{crate::grass::export_engine(&s,&out)}else{crate::grass::export(&s,&out,format,lod)}
}

pub(crate) fn capture_preview(s: Settings, args: &Value, path: &Path) -> Result<Value, String> {
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
        "3" => Some(3),
        _ => return Err("Preview LOD must be auto, 0, 1, 2 or 3".into()),
    };
    if lod == Some(3) && args.get("gameCards").and_then(Value::as_bool) == Some(false) {
        return Err("Preview LOD 3 is available only for game cards".into());
    }
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
    let mut skybox = preview::skybox(skybox_name)?;
    skybox.coverage = number("cloudCover", 1., skybox.coverage)?;
    let leaf_roughness = number("leafRoughness", 1., 0.55)?;
    if leaf_roughness < 0.05 { return Err("leafRoughness must be 0.05..1".into()); }
    let leaf_specular = args.get("leafSpecular").map(|v| v.as_bool().ok_or("leafSpecular must be boolean")).transpose()?.unwrap_or(true);
    let leaf_transmission = number("leafTransmission", 1., 0.8)?;
    let wind_strength = number("windStrength", 2., 1.)?;
    let wind_gust = number("windGust", 1., 0.2)?;
    let branch_sway = number("branchSway", 2., 1.)?;
    let branch_springiness = number("branchSpringiness", 1., 0.5)?;
    let leaf_flutter = number("leafFlutter", 2., 1.)?;
    let grass = args.get("grass").map(|v| serde_json::from_value::<crate::grass::Settings>(v.clone()).map_err(|e|e.to_string())).transpose()?;
    if let Some(g)=&grass {g.validate()?;}
    let mut camera=preview::camera_for(&s,forest,count,distance);
    if let Some(g)=&grass {
        let d=distance.unwrap_or(g.size.max(g.height_hint())*2.);let pitch=0.42_f32;let yaw=0.65_f32;
        camera.position=glam::Vec3::new(0.,g.height_hint()*0.35,0.)+glam::Vec3::new(yaw.sin()*pitch.cos(),pitch.sin(),yaw.cos()*pitch.cos())*d;
        camera.pitch=-pitch;
    }
    let frame = preview::capture(
        Request {
            camera,
            scene: SceneKey {
                grass: args.get("grass").map(|value| serde_json::from_value::<crate::grass::Settings>(value.clone()).map_err(|e|e.to_string())).transpose()?,
                settings: s,
                lod,
                forest,
                count,
                ratios,
                grid: forest == 0,
                bark: true,
                leaves: !args.get("structure").map(|v| v.as_bool().ok_or("structure must be boolean")).transpose()?.unwrap_or(false),
                game_cards: args.get("gameCards").map(|v| v.as_bool().ok_or("gameCards must be boolean")).transpose()?.unwrap_or(true),
                trim_cards: args.get("trimCards").map(|v| v.as_bool().ok_or("trimCards must be boolean")).transpose()?.unwrap_or(true),
                random_mix: args.get("randomMix").map(|v| v.as_bool().ok_or("randomMix must be boolean")).transpose()?.unwrap_or(true),
                forest_floor: args.get("forestFloor").map(|v| v.as_bool().ok_or("forestFloor must be boolean")).transpose()?.unwrap_or(true),
                leaf_specular,
                leaf_roughness,
                leaf_transmission,
            },
            debug,
            wind_time,
            wind_direction: number("windDirection", 360., 68.)?,
            wind_strength,
            wind_gust,
            branch_sway,
            branch_springiness,
            leaf_flutter,
            sky,
            skybox,
            lighting,
        },
        path,
    )?;
    Ok(
        json!({"path":path,"view":view,"skybox":skybox_name,"cloudCover":skybox.coverage,"leafSpecular":frame.scene.leaf_specular,"leafRoughness":frame.scene.leaf_roughness,"leafTransmission":frame.scene.leaf_transmission,"windStrength":wind_strength,"windGust":wind_gust,"branchSway":branch_sway,"branchSpringiness":branch_springiness,"leafFlutter":leaf_flutter,"timeOfDay":lighting.hours,"sunAzimuth":lighting.azimuth,"gameCards":frame.scene.game_cards,"trimCards":frame.scene.trim_cards,"clusters":frame.clusters,"drawnTriangles":frame.triangles,"gpuTimingsMs":frame.gpu_ms.map(|times| solstice_render::profiler::SECTIONS.iter().zip(times).map(|(name, time)| ((*name).to_owned(), time)).collect::<std::collections::BTreeMap<_,_>>())}),
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
            json!({"protocolVersion":version,"capabilities":{"tools":{"listChanged":false}},"serverInfo":{"name":"grove-studio","version":env!("CARGO_PKG_VERSION")},"instructions":"Use tree_presets, then tree_inspect or tree_generate. Generation is deterministic; units are meters; the Y axis is up."})
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
    fn grass_mcp_validates_and_exports_beneath_root() {
        let root=std::env::temp_dir().join(format!("grove-grass-mcp-{}",std::process::id()));
        assert!(call_tool("grass_generate", &json!({"output":"../escape.gltf"}), &root).is_err());
        assert!(call_tool("grass_inspect", &json!({"settings":{"size":100}}), &root).is_err());
        let result=call_tool("grass_generate", &json!({"settings":{"size":0.5,"density":0.1,"patchiness":0.0,"weeds":false},"format":"gltf","output":"patch.gltf","lod":2}), &root).unwrap();
        assert!(result["triangles"].as_u64().unwrap()>0);
        assert!(root.join("patch.gltf").is_file());
        assert!(call_tool("grass_generate", &json!({"output":"patch.gltf","lod":9}), &root).is_err());
    }

    #[test]
    fn mcp_budget_report_measures_game_faces() {
        let report = call_tool("tree_budget_report", &json!({"settings":{"species":"solstice-forest-beech"}, "trimCards":true}), Path::new("exports")).unwrap();
        let levels = report["levels"].as_array().unwrap();
        assert_eq!(levels.len(), 4);
        assert!(levels.iter().all(|l| l["two_sided_faces"].as_u64().unwrap() <= l["budget"].as_u64().unwrap()));
        assert!(call_tool("tree_budget_report", &json!({"trimCards":"yes"}), Path::new("exports")).is_err());
    }

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
        assert!(tools_list()["tools"].as_array().unwrap().iter().any(|tool| tool["name"] == "tree_randomize"));
        let args = json!({"settings":{"species":"solstice-oak"},"scope":"foliage","seed":23});
        assert_eq!(call_tool("tree_randomize", &args, Path::new("exports")).unwrap(), call_tool("tree_randomize", &args, Path::new("exports")).unwrap());
        assert!(call_tool("tree_randomize", &json!({"seed": 23, "scope": "invalid"}), Path::new("exports")).is_err());
        let one = call_tool("tree_randomize", &json!({"settings":{"species":"solstice-oak"},"field":"maturity","seed":23}), Path::new("exports")).unwrap();
        assert_eq!(one["field"], "maturity");
        assert_eq!(one["settings"]["density"], Settings::preset("solstice-oak").unwrap().density);
        let search = call_tool("tree_presets", &json!({"query":"smrk"}), Path::new("exports")).unwrap();
        assert!(search["presets"].as_array().unwrap().iter().any(|v| v["species"].as_str().unwrap().contains("smrk")));
        assert!(rpc(
            json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
            Path::new("exports")
        )
        .is_none());
    }
}
