import type {
  ApiResponse,
  App,
  ContainerStatus,
  Deployment,
  MaskedEnvVar,
} from '@/types/api'

class ApiError extends Error {
  status: number

  constructor(status: number, message: string) {
    super(message)
    this.status = status
  }
}

async function request<T>(path: string, options?: RequestInit): Promise<T> {
  const res = await fetch(`/api/v1${path}`, {
    headers: { 'Content-Type': 'application/json', ...options?.headers },
    ...options,
  })

  if (!res.ok) {
    let message = `HTTP ${res.status}`
    try {
      const json: ApiResponse<T> = await res.json()
      if (json.error) message = json.error
    } catch {
      // non-JSON error body (e.g. Traefik 502, nginx HTML)
    }
    throw new ApiError(res.status, message)
  }

  const json: ApiResponse<T> = await res.json()
  return json.data as T
}

// System
export const getHealth = () =>
  request<{ status: string; version: string }>('/system/health')
export const getSystemInfo = () =>
  request<{
    version: string
    docker_version: string
    app_count: number
    container_count: number
  }>('/system/info')

// Apps
export const listApps = () => request<App[]>('/apps')
export const getApp = (name: string) => request<App>(`/apps/${name}`)
export const createApp = (data: { name: string; port?: number }) =>
  request<App>('/apps', { method: 'POST', body: JSON.stringify(data) })
export const updateApp = (
  name: string,
  data: { port?: number; domain?: string },
) =>
  request<App>(`/apps/${name}`, { method: 'PUT', body: JSON.stringify(data) })
export const deleteApp = (name: string) =>
  request<void>(`/apps/${name}`, { method: 'DELETE' })

// Container lifecycle
export const startApp = (name: string) =>
  request<void>(`/apps/${name}/start`, { method: 'POST' })
export const stopApp = (name: string) =>
  request<void>(`/apps/${name}/stop`, { method: 'POST' })
export const restartApp = (name: string) =>
  request<void>(`/apps/${name}/restart`, { method: 'POST' })
export const getContainerStatus = (name: string) =>
  request<ContainerStatus>(`/apps/${name}/status`)
export const getContainerLogs = (name: string, tail = 200) =>
  request<string[]>(`/apps/${name}/logs?tail=${tail}`)

// Deployments
export const triggerDeploy = (name: string) =>
  request<{ deployment_id: string; status: string }>(
    `/apps/${name}/deploy`,
    { method: 'POST' },
  )
export const listDeployments = (name: string) =>
  request<Deployment[]>(`/apps/${name}/deployments`)
export const getDeployment = (name: string, id: string) =>
  request<Deployment>(`/apps/${name}/deployments/${id}`)
export const rollbackDeployment = (name: string, deploymentId: string) =>
  request<{ deployment_id: string; status: string }>(
    `/apps/${name}/deployments/${deploymentId}/rollback`,
    { method: 'POST' },
  )

// Env vars
export const listEnvVars = (name: string) =>
  request<MaskedEnvVar[]>(`/apps/${name}/env`)
export const bulkSetEnvVars = (name: string, vars: Record<string, string>) =>
  request<void>(`/apps/${name}/env`, {
    method: 'PUT',
    body: JSON.stringify(vars),
  })
export const deleteEnvVar = (name: string, key: string) =>
  request<void>(`/apps/${name}/env/${key}`, { method: 'DELETE' })

export { ApiError }
