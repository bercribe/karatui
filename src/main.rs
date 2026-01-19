use anyhow::Result;
use tokio::runtime::Runtime;

mod api;
mod app;

fn main() -> Result<()> {
    let rt = Runtime::new()?;
    let bookmarks = rt.block_on(async { api::fetch_bookmarks().await })?;
    let available_tags = rt.block_on(async { api::fetch_available_tags().await })?;
    let available_lists = rt.block_on(async { api::fetch_available_lists().await })?;

    let mut app = app::App::new(&bookmarks, &available_tags, &available_lists);
    ratatui::run(|terminal| app.run(terminal))
}
