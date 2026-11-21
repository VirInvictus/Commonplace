//! Commonplace CLI: `commonplace-cli`, the scriptable half of the app.
//! The verb machinery lives in the library half (`commonplace_cli`);
//! this binary parses, runs, prints, and maps the exit code (spec §6:
//! 0 success, 1 trouble, 2 usage; trouble and usage stay one clean
//! line on stderr).

use commonplace_cli::args;
use commonplace_cli::verbs::{self, Exit};

fn main() {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    if argv.first().map(String::as_str) == Some("--version")
        || argv.first().map(String::as_str) == Some("-V")
    {
        println!("commonplace-cli {}", commonplace_core::VERSION);
        return;
    }

    let invocation = match args::parse(&argv) {
        Ok(invocation) => invocation,
        Err(e) => {
            eprintln!("commonplace-cli: {e}");
            std::process::exit(2);
        }
    };

    let output = verbs::run(&invocation);
    for line in &output.stderr {
        eprintln!("{line}");
    }
    print!("{}", output.stdout);

    match output.exit {
        Exit::Ok => {}
        Exit::Trouble => std::process::exit(1),
        Exit::Usage => std::process::exit(2),
    }
}
