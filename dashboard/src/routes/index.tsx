import { createFileRoute, Link } from '@tanstack/react-router'
import { useQuery } from '@tanstack/react-query'
import { listApps, getSystemInfo } from '@/lib/api'
import { Badge } from '@/components/ui/badge'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import {
  Activity,
  Container,
  HardDrive,
  Server,
} from 'lucide-react'
import { statusDotColor, statusVariant, timeAgo } from '@/lib/utils'

export const Route = createFileRoute('/')({
  component: Overview,
})

function Overview() {
  const apps = useQuery({
    queryKey: ['apps'],
    queryFn: listApps,
    refetchInterval: 10_000,
  })
  const system = useQuery({
    queryKey: ['system-info'],
    queryFn: getSystemInfo,
    refetchInterval: 30_000,
  })

  return (
    <div className="space-y-6">
      {/* System stats */}
      <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
        <StatCard
          icon={Server}
          label="Apps"
          value={system.data?.app_count ?? '—'}
          loading={system.isLoading}
        />
        <StatCard
          icon={Container}
          label="Running"
          value={system.data?.container_count ?? '—'}
          loading={system.isLoading}
        />
        <StatCard
          icon={HardDrive}
          label="Docker"
          value={system.data?.docker_version ?? '—'}
          loading={system.isLoading}
          mono
        />
        <StatCard
          icon={Activity}
          label="API"
          value={system.data ? `v${system.data.version}` : '—'}
          loading={system.isLoading}
          mono
        />
      </div>

      {system.error && (
        <p className="text-sm text-destructive">
          Failed to connect to API
        </p>
      )}

      {/* App list */}
      <div>
        <h2 className="mb-3 text-lg font-semibold">Apps</h2>
        {apps.isLoading && (
          <p className="text-sm text-muted-foreground">Loading...</p>
        )}
        {apps.error && (
          <p className="text-sm text-destructive">
            Failed to load apps: {(apps.error as Error).message}
          </p>
        )}
        {apps.data?.length === 0 && (
          <p className="text-sm text-muted-foreground">
            No apps yet. Create one to get started.
          </p>
        )}
        {apps.data && apps.data.length > 0 && (
          <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
            {apps.data.map((app) => (
              <Link
                key={app.id}
                to="/apps/$name"
                params={{ name: app.name }}
                className="block"
              >
                <Card className="transition hover:border-ring/30">
                  <CardHeader className="flex flex-row items-center justify-between pb-2">
                    <div className="flex items-center gap-2">
                      <span
                        className={`inline-block h-2 w-2 rounded-full ${statusDotColor(app.status)}`}
                      />
                      <CardTitle className="font-mono text-sm">
                        {app.name}
                      </CardTitle>
                    </div>
                    <Badge variant={statusVariant(app.status)}>
                      {app.status}
                    </Badge>
                  </CardHeader>
                  <CardContent className="space-y-1 text-xs text-muted-foreground">
                    <p>{app.domain}</p>
                    <p>
                      Port {app.port}
                      {app.docker_image && (
                        <span className="ml-2 opacity-60">
                          {app.docker_image}
                        </span>
                      )}
                    </p>
                    <p className="opacity-60">
                      Updated {timeAgo(app.updated_at)}
                    </p>
                  </CardContent>
                </Card>
              </Link>
            ))}
          </div>
        )}
      </div>
    </div>
  )
}

function StatCard({
  icon: Icon,
  label,
  value,
  loading,
  mono,
}: {
  icon: React.ComponentType<{ className?: string }>
  label: string
  value: string | number
  loading?: boolean
  mono?: boolean
}) {
  return (
    <Card>
      <CardContent className="flex items-center gap-3 pt-4">
        <Icon className="h-4 w-4 text-muted-foreground" />
        <div>
          <p className="text-xs text-muted-foreground">{label}</p>
          <p className={`text-sm font-medium ${mono ? 'font-mono' : ''}`}>
            {loading ? '...' : value}
          </p>
        </div>
      </CardContent>
    </Card>
  )
}
