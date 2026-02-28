use bollard::Docker;
use bollard::image::{BuildImageOptions, TagImageOptions};
use futures_util::StreamExt;
use homelab_core::HomelabError;

/// Build a Docker image from a directory containing a Dockerfile.
///
/// Returns `(image_tag, build_log)` where image_tag is `homelab/<app>:<short_sha>`.
/// Also tags the image as `homelab/<app>:latest`.
pub async fn build_image(
    docker: &Docker,
    build_dir: &str,
    app_name: &str,
    commit_sha: &str,
) -> Result<(String, String), HomelabError> {
    let short_sha: String = commit_sha.chars().take(8).collect();
    let tag = format!("homelab/{app_name}:{short_sha}");

    // Create tar archive on a blocking thread (recursive FS walk)
    let tar_bytes = {
        let dir = build_dir.to_string();
        tokio::task::spawn_blocking(move || create_tar(&dir))
            .await
            .map_err(|e| HomelabError::Internal(format!("tar task panicked: {e}")))?
    }?;

    let options = BuildImageOptions {
        t: tag.clone(),
        dockerfile: "Dockerfile".to_string(),
        rm: true,
        ..Default::default()
    };

    let mut build_stream = docker.build_image(options, None, Some(tar_bytes.into()));
    let mut build_log = String::new();

    while let Some(result) = build_stream.next().await {
        match result {
            Ok(output) => {
                if let Some(stream) = &output.stream {
                    build_log.push_str(stream);
                }
                if let Some(error) = &output.error {
                    build_log.push_str(&format!("ERROR: {error}\n"));
                    return Err(HomelabError::Docker(format!("image build failed: {error}")));
                }
            }
            Err(e) => {
                return Err(HomelabError::Docker(format!(
                    "image build stream error: {e}"
                )));
            }
        }
    }

    // Tag as latest
    docker
        .tag_image(
            &tag,
            Some(TagImageOptions {
                repo: format!("homelab/{app_name}"),
                tag: "latest".to_string(),
            }),
        )
        .await
        .map_err(|e| HomelabError::Docker(format!("tag image: {e}")))?;

    tracing::info!(app = %app_name, tag = %tag, "image built");
    Ok((tag, build_log))
}

/// Create a tar archive of a directory in memory.
fn create_tar(dir: &str) -> Result<Vec<u8>, HomelabError> {
    let buf = Vec::new();
    let mut ar = tar::Builder::new(buf);
    ar.append_dir_all(".", dir)
        .map_err(|e| HomelabError::Internal(format!("create tar archive: {e}")))?;
    ar.into_inner()
        .map_err(|e| HomelabError::Internal(format!("finalize tar archive: {e}")))
}
