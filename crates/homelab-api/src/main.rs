mod error;
mod handlers;
mod middleware;
mod router;
mod state;

use state::{AppConfig, AppState};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    // Config from env
    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:data/homelab.db?mode=rwc".into());
    let git_repos_path =
        std::env::var("GIT_REPOS_PATH").unwrap_or_else(|_| "/opt/homelab/git-repos".into());
    let base_domain = std::env::var("BASE_DOMAIN").unwrap_or_else(|_| "lab.localhost".into());
    let internal_hook_secret =
        std::env::var("INTERNAL_HOOK_SECRET").unwrap_or_else(|_| "dev-secret".into());
    let services_path =
        std::env::var("SERVICES_PATH").unwrap_or_else(|_| "/opt/homelab/repo/services".into());
    let port: u16 = std::env::var("PORT")
        .unwrap_or_else(|_| "5170".into())
        .parse()?;

    // Secrets encryption (optional — if not set, secrets endpoints are disabled)
    let cipher = match std::env::var("SECRETS_ENCRYPTION_KEY") {
        Ok(key) => {
            let c = homelab_core::SecretsCipher::new(&key)?;
            tracing::info!("secrets encryption enabled");
            Some(c)
        }
        Err(_) => {
            tracing::info!("SECRETS_ENCRYPTION_KEY not set — secrets management disabled");
            None
        }
    };

    // ─── --sync-secrets mode ────────────────────────────────────────────────
    if std::env::args().any(|a| a == "--sync-secrets") {
        return sync_secrets(&database_url, cipher).await;
    }

    // Database
    let db = homelab_db::init_pool(&database_url).await?;
    homelab_db::run_migrations(&db).await?;

    // Docker
    let docker = homelab_docker::client::connect()?;
    tracing::info!(
        docker_version = ?docker.version().await.ok().and_then(|v| v.version),
        "docker connected"
    );

    // Ensure the homelab Docker network exists
    homelab_docker::network::ensure_network(&docker).await?;

    // Cloudflare (optional — if all 4 vars are set, enable tunnel management)
    let cloudflare = match (
        std::env::var("CLOUDFLARE_API_TOKEN").ok(),
        std::env::var("CLOUDFLARE_ACCOUNT_ID").ok(),
        std::env::var("CLOUDFLARE_TUNNEL_ID").ok(),
        std::env::var("CLOUDFLARE_ZONE_ID").ok(),
    ) {
        (Some(api_token), Some(account_id), Some(tunnel_id), Some(zone_id)) => {
            let cf = homelab_cloudflare::client::CloudflareClient::new(
                homelab_cloudflare::client::CloudflareConfig {
                    api_token,
                    account_id,
                    tunnel_id,
                    zone_id,
                    base_domain: base_domain.clone(),
                },
            )?;
            tracing::info!("cloudflare tunnel management enabled");
            Some(cf)
        }
        _ => {
            tracing::info!("cloudflare not configured — tunnel management disabled");
            None
        }
    };

    // Build state
    let state = AppState {
        db,
        docker,
        cloudflare,
        cipher,
        config: AppConfig {
            git_repos_path,
            base_domain,
            internal_hook_secret,
            api_port: port,
            services_path,
        },
    };

    let app = router::build(state);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;
    tracing::info!("listening on port {port}");

    axum::serve(listener, app).await?;
    Ok(())
}

async fn sync_secrets(
    database_url: &str,
    cipher: Option<homelab_core::SecretsCipher>,
) -> anyhow::Result<()> {
    let cipher = cipher
        .ok_or_else(|| anyhow::anyhow!("SECRETS_ENCRYPTION_KEY is required for --sync-secrets"))?;

    let db = homelab_db::init_pool(database_url).await?;
    homelab_db::run_migrations(&db).await?;

    let services = homelab_db::service_repo::list(&db).await?;
    tracing::info!(count = services.len(), "syncing service secrets");

    for service in &services {
        let rows = homelab_db::service_secret_repo::get_by_service(&db, &service.id).await?;
        if rows.is_empty() {
            continue;
        }

        let vars: Vec<(String, String)> = rows
            .into_iter()
            .map(|row| {
                let value = cipher.decrypt(&row.encrypted_value, &row.nonce)?;
                Ok((row.key, value))
            })
            .collect::<Result<_, homelab_core::HomelabError>>()?;

        let compose_dir = std::path::PathBuf::from(&service.compose_path);
        if compose_dir.exists() {
            homelab_docker::compose::write_env_file(&compose_dir, &vars).await?;
            tracing::info!(service = %service.name, secrets = vars.len(), "synced");
        } else {
            tracing::warn!(
                service = %service.name,
                path = %compose_dir.display(),
                "compose dir not found, skipping"
            );
        }
    }

    tracing::info!("secrets sync complete");
    Ok(())
}
