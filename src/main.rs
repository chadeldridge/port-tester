use hosts::Host;
use std::sync::Arc;

mod args;
mod hosts;
mod metrics;

fn main() {
    // Set up Ctrl-C handler to print report on interrupt. We need to create the host object first
    // so we can access its metrics in the handler.
    let mut host = Host::default();
    let metrics_clone = Arc::clone(&host.metrics);
    ctrlc::set_handler(move || {
        println!("\nInterrupted! Generating report...");
        metrics_clone.lock().unwrap().report();
        std::process::exit(0);
    })
    .expect("Error setting Ctrl-C handler");

    if host.args.host.is_empty() {
        eprintln!("Error: Host is required.");
        args::Args::print_help();
        std::process::exit(1);
    }

    // Format the target address for use with TcpStream.
    //let address = format!("{}:{}", args.host, args.port);
    host.address = host.fortmat_address();

    // Get an iterator for the number of attempts. If count is 0, it will be infinite.
    let iter = if host.args.count == 0 {
        std::iter::repeat(()).take(u32::MAX as usize)
    } else {
        std::iter::repeat(()).take(host.args.count as usize)
    };

    for i in iter.enumerate().map(|(i, _)| i as u32 + 1) {
        // Do not report the attempt number if in silent mode or if we are only doing one attempt.
        if !host.args.silent && host.args.count != 1 {
            print!("{} ", i);
        }
        // Attempt to connect to the target and record metrics.
        host.attempt_connect();

        // Print intermediate report if report_interval is set.
        // If the count is reached, the final report will be printed after the loop.
        if host.args.report_interval > 0
            && i % host.args.report_interval == 0
            && (host.args.count == 0 || i < host.args.count)
        {
            print!("Intermediate report: ");
            host.metrics.lock().unwrap().report();
        }

        // Sleep between attempts unless this is the last attempt.
        if host.args.count == 0 || i < host.args.count {
            std::thread::sleep(std::time::Duration::from_secs(host.args.interval));
        }
    }

    // Do not give the final report for a single attempt.
    if host.args.count != 1 {
        host.metrics.lock().unwrap().report();
    }
}
