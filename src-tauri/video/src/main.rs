//! `editlicious-render` – the video export without the app (design 017 E6), for pipelines and CI.

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    std::process::exit(ed_video::cli::main(&args));
}
