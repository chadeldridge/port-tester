use cli::{Args, Cli};
use port_tester::connectors::port_open::*;
use port_tester::core::error::*;
use port_tester::{Host, Verbosity};

use env_logger::Env;
use log::{debug, info};
use std::sync::{Arc, Mutex};

mod cli;

const DEFAULT_LOG_LEVEL: &str = "error";

fn main() {
    let cli = Cli::new(Args::new());
    setup_logger(&cli.verbose);

    let mut verbose = &Verbosity::default();
    if let Some(v) = &cli.verbose {
        verbose = v;
    }
    debug!("verbosity: {}", verbose);

    if cli.args.host.is_empty() {
        eprintln!("Error: Host is required.");
        Cli::print_help();
        std::process::exit(1);
    }

    // Set up Ctrl-C handler to print report on interrupt. We need to create the host object first
    // so we can access its metrics in the handler.
    let host = Arc::new(Mutex::new(match Host::new(&cli.args.host, cli.args.port) {
        Ok(h) => h,
        Err(e) => exit_handler(&e),
    }));
    info!("host: {}", host.lock().unwrap().name());

    // Clone metrics so we have access to it in the ctrlc handler.
    // let metrics_clone = Arc::clone(&host.metrics);
    let cli_clone = cli.clone();
    let host_clone = Arc::clone(&host);
    // Create a handler that will attempt to print a metrics report when we receive a Ctrl-C.
    ctrlc::set_handler(move || {
        //println!("\nInterrupted! Generating report...");
        print_report(&cli_clone, &host_clone.lock().unwrap());
        std::process::exit(0);
    })
    .expect("Error setting Ctrl-C handler");

    // Get an iterator for the number of attempts. If count is 0, it will be infinite.
    let iter = if cli.args.count == 0 {
        debug!("attempts: infinite");
        std::iter::repeat(()).take(u32::MAX as usize)
    } else {
        let c = cli.args.count as usize;
        debug!("attempts: {}", c);
        std::iter::repeat(()).take(c)
    };

    for i in iter.enumerate().map(|(i, _)| i as u32 + 1) {
        debug!(
            "attempt: {}, ip: {}, port: {}, timeout: {}",
            i,
            host.lock().unwrap().ip(),
            host.lock().unwrap().port(),
            cli.args.timeout
        );

        // Connect to the target and record metrics.
        //handle_results(connect(&host, cli.args.timeout), verbose, cli.args.count);
        connect(i, &mut host.lock().unwrap(), cli.args.timeout);

        // Use a block so the MutexGuard is dropped before the intermediate report and sleep,
        // otherwise those sites deadlock trying to re-acquire the same lock.
        let (display_str, is_err) = {
            let h = host.lock().unwrap();
            let mr = h.metrics().result(i).unwrap();
            let status = mr.status();
            let display = match cli.args.count {
                1 => status.to_string_with_verbosity(verbose),
                _ => mr.to_string_with_verbosity(verbose),
            };
            (display, status.is_err())
        };

        if !cli.args.silent && !cli.args.json {
            println!("{}", display_str);
        }

        if cli.args.count == 1 {
            if is_err {
                std::process::exit(1);
            } else {
                std::process::exit(0);
            }
        }

        // Print intermediate report if report_interval is set.
        // If the count is reached, the final report will be printed after the loop.
        if !cli.args.json
            && cli.args.report_interval > 0
            && i % cli.args.report_interval == 0
            && (cli.args.count == 0 || i < cli.args.count)
        {
            print!("Intermediate report: ");
            println!("{}", host.lock().unwrap().metrics().report());
        }

        // Sleep between attempts unless this is the last attempt.
        if cli.args.count == 0 || i < cli.args.count {
            debug!("sleep: {}s", cli.args.interval);
            std::thread::sleep(std::time::Duration::from_secs(cli.args.interval));
        }
    }

    print_report(&cli, &host.lock().unwrap());
}

fn print_report(cli: &Cli, host: &Host) {
    debug!("connection attempts complete, print final report");
    if cli.args.json {
        let h_json_string = match host.to_json_string() {
            Ok(j) => j,
            Err(e) => {
                exit_handler(&e);
            }
        };
        println!("{}", h_json_string);
        return;
    }

    // Do not give the final report for a single attempt.
    if !cli.args.json && cli.args.count != 1 {
        println!("{}", host.metrics().report());
    }
}

fn setup_logger(level: &Option<Verbosity>) {
    // Prioritize log levels: cli flag > env var > default
    let env = Env::default()
        .filter_or("PT_LOG_LEVEL", DEFAULT_LOG_LEVEL)
        .write_style_or("PT_LOG_STYLE", "always");
    let mut builder = env_logger::Builder::from_env(env);
    if let Some(v) = level {
        builder.filter(None, v.to_filter_level());
    }
    builder.init();
}

/// Handle error and exit program.
fn exit_handler(error: &Error) -> ! {
    handle_error(error);
    if error.is_print_help() {
        cli::Cli::print_help();
    }
    debug!("Exiting with code {:?}", error.code());
    std::process::exit(error.code().unwrap_or(1));
}
