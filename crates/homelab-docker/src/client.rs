use bollard::Docker;
use homelab_core::HomelabError;

pub fn connect() -> Result<Docker, HomelabError> {
    Docker::connect_with_socket_defaults()
        .map_err(|e| HomelabError::Docker(format!("failed to connect to Docker: {e}")))
}
