use bollard::Docker;
use homelab_cloudflare::client::CloudflareClient;
use sqlx::SqlitePool;

#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub docker: Docker,
    pub cloudflare: Option<CloudflareClient>,
    pub config: AppConfig,
}

#[derive(Clone)]
pub struct AppConfig {
    pub git_repos_path: String,
    pub base_domain: String,
    pub internal_hook_secret: String,
    pub api_port: u16,
}
