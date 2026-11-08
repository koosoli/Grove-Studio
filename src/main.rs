mod agent;
mod catalog;
mod gui;
mod log;
mod poseidon;
mod preview;
mod reference;
mod tree;
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    log::install_panic_hook();
    log::log(format!("start: args {args:?}"));
    if let Some(result) = agent::dispatch(&args) {
        if let Err(e) = result {
            eprintln!("Grove Studio: {e}");
            std::process::exit(1)
        }
        return;
    }
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--classic" {
            i += 1;
            continue;
        }
        if ![
            "--screenshot",
            "--self-test",
            "--preset",
            "--config",
            "--forest",
            "--preview-view",
        ]
        .contains(&args[i].as_str())
            || i + 1 >= args.len()
            || args[i + 1].starts_with("--")
        {
            eprintln!(
                "Grove Studio: invalid or incomplete option {}. Use --help.",
                args[i]
            );
            std::process::exit(1);
        }
        i += 2;
    }
    for result in [
        agent::option(&args, "--forest").map(|value| preview::forest_mask(&value).map(|_| ())),
        agent::option(&args, "--preview-view").map(|value| {
            solstice_render::DebugView::from_name(&value)
                .map(|_| ())
                .ok_or_else(|| format!("Unknown preview view {value:?}"))
        }),
    ]
    .into_iter()
    .flatten()
    {
        if let Err(error) = result {
            eprintln!("Grove Studio: {error}");
            std::process::exit(1);
        }
    }
    if args.iter().any(|arg| arg == "--classic")
        && (agent::option(&args, "--forest").is_some()
            || agent::option(&args, "--preview-view").is_some())
    {
        eprintln!("Grove Studio: forest and diagnostic views require the Solstice renderer.");
        std::process::exit(1);
    }
    // Keep CLI/MCP streams intact; a visual launch does not need a console window.
    #[cfg(windows)]
    unsafe {
        #[link(name = "kernel32")]
        extern "system" {
            fn FreeConsole() -> i32;
        }
        FreeConsole();
    }
    let conf = macroquad::conf::Conf {
        miniquad_conf: macroquad::miniquad::conf::Conf {
            window_title: "Grove Studio | Tree workshop".into(),
            window_width: 1440,
            window_height: 900,
            high_dpi: true,
            sample_count: 4,
            ..Default::default()
        },
        draw_call_vertex_capacity: 65535,
        draw_call_index_capacity: 200000,
        ..Default::default()
    };
    macroquad::Window::from_config(conf, gui::run(args));
}
