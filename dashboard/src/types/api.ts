export type AppStatus = "created" | "building" | "running" | "stopped" | "failed";
export type DeployStatus = "pending" | "building" | "deploying" | "succeeded" | "failed";

export interface App {
  id: string;
  name: string;
  domain: string;
  git_repo_path: string;
  docker_image: string;
  port: number;
  status: AppStatus;
  created_at: string;
  updated_at: string;
}

export interface Deployment {
  id: string;
  app_id: string;
  commit_sha: string;
  image_tag: string;
  status: DeployStatus;
  build_log: string | null;
  started_at: string;
  finished_at: string | null;
}

export interface ContainerStatus {
  container_id: string;
  state: string;
  uptime: string;
  cpu_percent: number;
  memory_mb: number;
  memory_limit_mb: number;
}

export interface MaskedEnvVar {
  key: string;
  value: string;
}

export interface SystemHealth {
  status: string;
  version: string;
}

export interface SystemInfo {
  version: string;
  docker_version: string;
  app_count: number;
  container_count: number;
}

export interface ApiResponse<T> {
  success: boolean;
  data?: T;
  error?: string;
}
