use homelab_core::HomelabError;
use std::path::Path;
use tokio::process::Command;

/// Initialize a bare git repo at the given path.
pub async fn init_bare(path: &str) -> Result<(), HomelabError> {
    let parent = Path::new(path)
        .parent()
        .ok_or_else(|| HomelabError::Internal("invalid repo path".into()))?;

    tokio::fs::create_dir_all(parent)
        .await
        .map_err(|e| HomelabError::Internal(format!("create git parent dir: {e}")))?;

    let output = Command::new("git")
        .args(["init", "--bare", path])
        .output()
        .await
        .map_err(|e| HomelabError::Internal(format!("git init: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(HomelabError::Internal(format!(
            "git init failed: {stderr}"
        )));
    }

    tracing::info!(path = %path, "bare repo initialized");
    Ok(())
}

/// Checkout a specific commit from a bare repo to a destination directory.
pub async fn checkout(repo_path: &str, commit_sha: &str, dest: &str) -> Result<(), HomelabError> {
    tokio::fs::create_dir_all(dest)
        .await
        .map_err(|e| HomelabError::Internal(format!("create checkout dir: {e}")))?;

    let output = Command::new("git")
        .arg(format!("--work-tree={dest}"))
        .arg(format!("--git-dir={repo_path}"))
        .args(["checkout", "-f", commit_sha])
        .output()
        .await
        .map_err(|e| HomelabError::Internal(format!("git checkout: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(HomelabError::Internal(format!(
            "git checkout failed: {stderr}"
        )));
    }

    // Verify Dockerfile exists
    if !Path::new(dest).join("Dockerfile").exists() {
        return Err(HomelabError::InvalidInput(
            "no Dockerfile found in repository root".into(),
        ));
    }

    tracing::info!(repo = %repo_path, sha = %commit_sha, "code checked out");
    Ok(())
}

/// Get the HEAD commit SHA from a bare repo.
pub async fn get_head_sha(repo_path: &str) -> Result<String, HomelabError> {
    let output = Command::new("git")
        .args(["--git-dir", repo_path, "rev-parse", "HEAD"])
        .output()
        .await
        .map_err(|e| HomelabError::Internal(format!("git rev-parse: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(HomelabError::Internal(format!(
            "git rev-parse failed: {stderr}"
        )));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Remove a bare git repo directory.
pub async fn remove(path: &str) -> Result<(), HomelabError> {
    let p = Path::new(path);
    if p.exists() {
        tokio::fs::remove_dir_all(p)
            .await
            .map_err(|e| HomelabError::Internal(format!("remove git repo: {e}")))?;
        tracing::info!(path = %path, "bare repo removed");
    }
    Ok(())
}
