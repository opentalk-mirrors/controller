// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use clap::Parser as _;
use opentalk_controller_service::Whatever;
use snafu::{ErrorCompat as _, Report};

mod cli;

type Result<T, E = Whatever> = std::result::Result<T, E>;

#[actix_web::main]
async fn main() {
    let args = cli::Args::parse();

    if let Err(err) = args.exec().await {
        dump_err(err);
        std::process::exit(-1);
    }
}

fn dump_err(err: Whatever) {
    let show_backtrace = std::env::var("RUST_BACKTRACE").is_ok_and(|v| v != "0");

    let backtrace = if show_backtrace {
        err.backtrace()
            .map(|e| format!("\nBacktrace:\n{e}"))
            .unwrap_or_else(|| "No backtrace available".to_string())
    } else {
        "NOTE: run with `RUST_BACKTRACE=1` environment variable to display a backtrace".to_string()
    };

    let report = Report::from_error(err);

    let message = format!("Error: {report}{backtrace}");

    if log::log_enabled!(log::Level::Error) {
        log::error!("{message}");
    } else {
        eprintln!("{message}");
    }
}
