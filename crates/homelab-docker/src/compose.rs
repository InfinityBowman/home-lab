use homelab_core::HomelabError;
use std::path::Path;
use tokio::fs;
use tokio::process::Command;

pub async fn write_env_file(
    compose_dir: &Path,
    vars: &[(String, String)],
) -> Result<(), HomelabError> {
    let env_path = compose_dir.join(".env");
    let content: String = vars
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join("\n");

    fs::write(&env_path, content)
        .await
        .map_err(|e| HomelabError::Internal(format!("write .env: {e}")))?;

    tracing::info!(path = %env_path.display(), "wrote .env file");
    Ok(())
}

pub async fn restart_compose(compose_dir: &Path) -> Result<(), HomelabError> {
    let output = Command::new("docker")
        .args(["compose", "up", "-d"])
        .current_dir(compose_dir)
        .output()
        .await
        .map_err(|e| HomelabError::Docker(format!("spawn docker compose: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(HomelabError::Docker(format!(
            "docker compose up failed: {stderr}"
        )));
    }

    tracing::info!(dir = %compose_dir.display(), "docker compose restarted");
    Ok(())
}
