use bollard::Docker;
use bollard::container::LogsOptions;
use futures_util::StreamExt;
use homelab_core::HomelabError;

use crate::containers::container_name;

pub async fn get_logs(
    docker: &Docker,
    app_name: &str,
    tail: u64,
) -> Result<Vec<String>, HomelabError> {
    let name = container_name(app_name);

    let opts = LogsOptions::<String> {
        stdout: true,
        stderr: true,
        tail: tail.to_string(),
        ..Default::default()
    };

    let mut stream = docker.logs(&name, Some(opts));
    let mut lines = Vec::new();

    while let Some(result) = stream.next().await {
        match result {
            Ok(output) => lines.push(output.to_string()),
            Err(e) => {
                return Err(HomelabError::Docker(format!("log stream error: {e}")));
            }
        }
    }

    Ok(lines)
}
