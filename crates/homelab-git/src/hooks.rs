use homelab_core::HomelabError;
use std::path::Path;

/// Write the post-receive hook into a bare git repo.
/// The hook curls the PaaS API when code is pushed to main.
pub async fn write_post_receive(
    repo_path: &str,
    app_name: &str,
    hook_secret: &str,
    api_port: u16,
) -> Result<(), HomelabError> {
    // Defense-in-depth: validate app_name is safe for embedding in a shell script
    if app_name.is_empty()
        || !app_name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return Err(HomelabError::InvalidInput(
            "app name contains invalid characters for hook script".into(),
        ));
    }

    let hooks_dir = Path::new(repo_path).join("hooks");
    tokio::fs::create_dir_all(&hooks_dir)
        .await
        .map_err(|e| HomelabError::Internal(format!("create hooks dir: {e}")))?;

    let hook_path = hooks_dir.join("post-receive");
    let script = format!(
        r#"#!/bin/bash
while read oldrev newrev ref; do
  if [ "$ref" = "refs/heads/main" ]; then
    curl -s -X POST \
      -H "Content-Type: application/json" \
      -H "Authorization: Bearer {hook_secret}" \
      -d "{{\\"ref\\":\\"$ref\\",\\"commit_sha\\":\\"$newrev\\"}}" \
      http://localhost:{api_port}/hooks/git/{app_name}
  fi
done
"#
    );

    tokio::fs::write(&hook_path, script)
        .await
        .map_err(|e| HomelabError::Internal(format!("write hook: {e}")))?;

    // Make executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o755);
        tokio::fs::set_permissions(&hook_path, perms)
            .await
            .map_err(|e| HomelabError::Internal(format!("chmod hook: {e}")))?;
    }

    tracing::info!(repo = %repo_path, app = %app_name, "post-receive hook written");
    Ok(())
}
