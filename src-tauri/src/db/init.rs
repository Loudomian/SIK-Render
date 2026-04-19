use anyhow::Result;
use tauri::AppHandle;

pub async fn init(_app: &AppHandle) -> Result<()> {
    // TODO: create SQLite pool, run migrations, store in AppState
    // sqlx::migrate!("./migrations").run(&pool).await?;
    log::info!("DB initialized (stub)");
    Ok(())
}
