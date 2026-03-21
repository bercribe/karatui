use anyhow::{Context, Result};
use tokio::runtime::Runtime;

mod api;
mod app;
mod conf;

fn main() -> Result<()> {
    let config = conf::get_config()?;

    let rt = Runtime::new()?;
    let bookmarks = rt
        .block_on(async { api::fetch_bookmarks(&config).await })
        .context("Failed to fetch bookmarks")?;
    let available_tags = rt
        .block_on(async { api::fetch_available_tags(&config).await })
        .context("Failed to fetch tags")?;
    let available_lists = rt
        .block_on(async { api::fetch_available_lists(&config).await })
        .context("Failed to fetch lists")?;

    let mut app = app::App::new(config, &bookmarks, &available_tags, &available_lists);
    ratatui::run(|terminal| app.run(terminal))
}
