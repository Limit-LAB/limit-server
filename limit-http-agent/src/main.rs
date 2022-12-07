#![feature(exitcode_exit_method)]

use std::process::ExitCode;

use limit_deps::{anyhow::Result, *};

mod_use::mod_use![server, config];

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("Error: {e}");
        ExitCode::FAILURE.exit_process();
    }
}

async fn run() -> Result<()> {
    let config = Config::get();
    server().await?;
    Ok(())
}
