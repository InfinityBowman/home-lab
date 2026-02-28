use std::collections::HashMap;

pub fn traefik_labels(app_name: &str, domain: &str, port: i64) -> HashMap<String, String> {
    let router = format!("homelab-{app_name}");

    HashMap::from([
        ("traefik.enable".into(), "true".into()),
        (
            format!("traefik.http.routers.{router}.rule"),
            format!("Host(`{domain}`)"),
        ),
        (
            format!("traefik.http.routers.{router}.entrypoints"),
            "web".into(),
        ),
        (
            format!("traefik.http.services.{router}.loadbalancer.server.port"),
            port.to_string(),
        ),
        (
            format!("traefik.http.routers.{router}.middlewares"),
            "secure-chain@file".into(),
        ),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_contain_correct_host_rule() {
        let labels = traefik_labels("my-app", "my-app.lab.example.com", 3000);
        assert_eq!(
            labels["traefik.http.routers.homelab-my-app.rule"],
            "Host(`my-app.lab.example.com`)"
        );
    }

    #[test]
    fn labels_contain_correct_port() {
        let labels = traefik_labels("my-app", "my-app.lab.example.com", 8080);
        assert_eq!(
            labels["traefik.http.services.homelab-my-app.loadbalancer.server.port"],
            "8080"
        );
    }

    #[test]
    fn labels_enable_traefik() {
        let labels = traefik_labels("foo", "foo.lab.dev", 3000);
        assert_eq!(labels["traefik.enable"], "true");
    }

    #[test]
    fn labels_has_five_entries() {
        let labels = traefik_labels("app", "app.lab.dev", 3000);
        assert_eq!(labels.len(), 5);
    }

    #[test]
    fn labels_attach_secure_chain_middleware() {
        let labels = traefik_labels("my-app", "my-app.lab.example.com", 3000);
        assert_eq!(
            labels["traefik.http.routers.homelab-my-app.middlewares"],
            "secure-chain@file"
        );
    }
}
