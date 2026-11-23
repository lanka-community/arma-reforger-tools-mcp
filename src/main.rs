//! The `reforger` MCP server for Arma Reforger modding.

// region:    --- Modules

mod answer_size;
#[path = "api-docs/mod.rs"]
mod api_docs;
#[path = "game-files/mod.rs"]
mod game_files;
mod guid;
mod host_path;
mod mod_folder;
#[path = "reference-mods/mod.rs"]
mod reference_mods;
mod server;
mod setup;
mod steam;
mod wiki;
mod workbench;
mod workbench_documents;
mod wsl;

#[cfg(test)]
mod test_support;

// endregion: --- Modules

use std::io::Write as _;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // `reforger setup` is a command for a person in a terminal, not the MCP server, so only it
    // gets stdout. It writes the settings file only after the person types `y`.
    if std::env::args().nth(1).as_deref() == Some("setup") {
        if let Err(err) = setup::run_from_env(&mut std::io::stdin().lock(), &mut std::io::stdout())
        {
            writeln!(std::io::stderr(), "{err}")?;
            std::process::exit(1);
        }
        return Ok(());
    }
    // Logs go to stderr, because stdout carries the MCP messages. Claude Code saves them in
    // ~/.cache/claude-cli-nodejs/<project>/mcp-logs-plugin-reforger-reforger/.
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_max_level(tracing_subscriber::filter::LevelFilter::INFO)
        .init();
    server::run().await
}
