// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // `editlicious render …` renders a script or recording as a video without a window
    // (design 017 E6)
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("render") {
        std::process::exit(ed_video::cli::main(&args[1..]));
    }
    ed_lib::run();
}
