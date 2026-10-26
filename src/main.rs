//! btrbk-nas: what the btrbk backups a Synology NAS receives look like.
//!
//! One binary, two faces: run by DSM's web server it is the CGI behind the
//! package interface; run from a shell it prints the same data.

mod api;
mod auth;
mod browse;
mod btrfs;
mod caps;
mod cgi;
mod config;
mod monitor;
mod report;
mod snapshot;
mod source;

use std::path::Path;
use std::process::ExitCode;

const USAGE: &str = "usage: btrbk-nas setup | check [--dry-run] | notify-test | daemon | scan DIR | discover | version
  setup     give this binary the capabilities it needs (as root, once
            after every install or upgrade of the package)
  check     run the periodic check once and print what it notifies;
            --dry-run sends nothing and leaves the saved state alone
  notify-test  send the test notification through DSM
  daemon    the periodic check, started by the package
Run by DSM's web server (GATEWAY_INTERFACE set) it answers the API.";

fn main() -> ExitCode {
    if std::env::var_os("GATEWAY_INTERFACE").is_some() {
        return match api::serve() {
            Ok(()) => ExitCode::SUCCESS,
            Err(_) => ExitCode::FAILURE,
        };
    }

    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.iter().map(String::as_str).collect::<Vec<_>>().as_slice() {
        ["scan", dir] => match snapshot::scan(Path::new(dir)) {
            Ok(scan) => {
                if let Some(e) = scan.metadata_error {
                    eprintln!("cannot read btrfs metadata: {e}");
                }
                println!("{}", serde_json::to_string_pretty(&scan.groups).unwrap_or_default());
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("{dir}: {e}");
                ExitCode::FAILURE
            }
        },
        ["discover"] => {
            for dir in snapshot::discover(Path::new("/")) {
                println!("{}", dir.display());
            }
            ExitCode::SUCCESS
        }
        ["setup"] => setup(),
        ["check", rest @ ..] if rest.iter().all(|a| *a == "--dry-run") => {
            let config = match config::Config::load(&config::Config::path()) {
                Ok(config) => config,
                Err(e) => {
                    eprintln!("cannot read settings: {e}");
                    return ExitCode::FAILURE;
                }
            };
            match monitor::check(&config, !rest.is_empty()) {
                Ok(events) => {
                    println!("{}", serde_json::to_string_pretty(&events).unwrap_or_default());
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("check failed: {e}");
                    ExitCode::FAILURE
                }
            }
        }
        ["daemon"] => monitor::run(),
        ["notify-test"] => match config::Config::load(&config::Config::path())
            .map_err(|e| std::io::Error::other(format!("cannot read settings: {e}")))
            .and_then(|config| monitor::send(&monitor::Event::test("btrbk-nas notify-test"), &config.notifications))
        {
            Ok(()) => {
                println!("test notification handed to DSM");
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("cannot notify: {e}");
                ExitCode::FAILURE
            }
        },
        ["version"] => {
            println!("btrbk-nas {}", api::VERSION);
            ExitCode::SUCCESS
        }
        _ => {
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
    }
}

fn setup() -> ExitCode {
    let binary = match std::env::current_exe() {
        Ok(path) => path,
        Err(e) => {
            eprintln!("cannot locate this binary: {e}");
            return ExitCode::FAILURE;
        }
    };
    match caps::install(&binary) {
        Ok(()) => {
            println!("{}: cap_sys_admin and cap_dac_read_search set", binary.display());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{}: {e} (run it as root: sudo {} setup)", binary.display(), binary.display());
            ExitCode::FAILURE
        }
    }
}
