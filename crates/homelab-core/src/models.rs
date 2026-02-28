use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

// ─── App ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct App {
    pub id: String,
    pub name: String,
    pub domain: String,
    pub git_repo_path: String,
    pub docker_image: String,
    pub port: i64,
    pub status: AppStatus,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AppStatus {
    Created,
    Building,
    Running,
    Stopped,
    Failed,
}

impl fmt::Display for AppStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Created => write!(f, "created"),
            Self::Building => write!(f, "building"),
            Self::Running => write!(f, "running"),
            Self::Stopped => write!(f, "stopped"),
            Self::Failed => write!(f, "failed"),
        }
    }
}

impl FromStr for AppStatus {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "created" => Ok(Self::Created),
            "building" => Ok(Self::Building),
            "running" => Ok(Self::Running),
            "stopped" => Ok(Self::Stopped),
            "failed" => Ok(Self::Failed),
            other => Err(format!("unknown app status: {other}")),
        }
    }
}

// ─── Deployment ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Deployment {
    pub id: String,
    pub app_id: String,
    pub commit_sha: String,
    pub image_tag: String,
    pub status: DeployStatus,
    pub build_log: Option<String>,
    pub started_at: String,
    pub finished_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DeployStatus {
    Pending,
    Building,
    Deploying,
    Succeeded,
    Failed,
}

impl fmt::Display for DeployStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Building => write!(f, "building"),
            Self::Deploying => write!(f, "deploying"),
            Self::Succeeded => write!(f, "succeeded"),
            Self::Failed => write!(f, "failed"),
        }
    }
}

impl FromStr for DeployStatus {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pending" => Ok(Self::Pending),
            "building" => Ok(Self::Building),
            "deploying" => Ok(Self::Deploying),
            "succeeded" => Ok(Self::Succeeded),
            "failed" => Ok(Self::Failed),
            other => Err(format!("unknown deploy status: {other}")),
        }
    }
}

// ─── EnvVar ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvVar {
    pub id: String,
    pub app_id: String,
    pub key: String,
    pub value: String,
    pub created_at: String,
}

// ─── AuditEntry ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub id: i64,
    pub app_id: Option<String>,
    pub action: String,
    pub details: Option<String>,
    pub created_at: String,
}

// ─── API Request/Response types ─────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateAppRequest {
    pub name: String,
    #[serde(default = "default_port")]
    pub port: i64,
}

fn default_port() -> i64 {
    3000
}

#[derive(Debug, Deserialize)]
pub struct UpdateAppRequest {
    pub port: Option<i64>,
    pub domain: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_status_round_trip() {
        for status in [
            AppStatus::Created,
            AppStatus::Building,
            AppStatus::Running,
            AppStatus::Stopped,
            AppStatus::Failed,
        ] {
            let s = status.to_string();
            let parsed: AppStatus = s.parse().unwrap();
            assert_eq!(parsed, status);
        }
    }

    #[test]
    fn deploy_status_round_trip() {
        for status in [
            DeployStatus::Pending,
            DeployStatus::Building,
            DeployStatus::Deploying,
            DeployStatus::Succeeded,
            DeployStatus::Failed,
        ] {
            let s = status.to_string();
            let parsed: DeployStatus = s.parse().unwrap();
            assert_eq!(parsed, status);
        }
    }

    #[test]
    fn app_status_rejects_unknown() {
        assert!("unknown".parse::<AppStatus>().is_err());
    }

    #[test]
    fn deploy_status_rejects_unknown() {
        assert!("unknown".parse::<DeployStatus>().is_err());
    }

    #[test]
    fn default_port_is_3000() {
        let json = r#"{"name":"test"}"#;
        let req: CreateAppRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.port, 3000);
    }
}
