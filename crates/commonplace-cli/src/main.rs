//! Commonplace CLI: `commonplace-cli`, the scriptable half of the app.
//! Real verbs land with the CLI phases; the skeleton answers --version
//! so the crate, app, and engine all link.

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("--version") | Some("-V") => println!("commonplace-cli {}", commonplace_core::VERSION),
        _ => {
            eprintln!(
                "commonplace-cli {}: no verbs yet; see roadmap.md",
                commonplace_core::VERSION
            );
            std::process::exit(2);
        }
    }
}
