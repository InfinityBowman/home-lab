import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { getContainerStatus, startApp, stopApp, restartApp } from '@/lib/api'
import type { App } from '@/types/api'
import { Button } from '@/components/ui/button'
import { Card, CardContent } from '@/components/ui/card'
import { Play, Square, RotateCw } from 'lucide-react'

export function AppStatusPanel({ app }: { app: App }) {
  const queryClient = useQueryClient()
  const invalidate = () => {
    queryClient.invalidateQueries({ queryKey: ['app', app.name] })
    queryClient.invalidateQueries({ queryKey: ['container-status', app.name] })
  }

  const status = useQuery({
    queryKey: ['container-status', app.name],
    queryFn: () => getContainerStatus(app.name),
    refetchInterval: 5_000,
    enabled: app.status === 'running',
    retry: false,
  })

  const start = useMutation({
    mutationFn: () => startApp(app.name),
    onSuccess: invalidate,
  })
  const stop = useMutation({
    mutationFn: () => stopApp(app.name),
    onSuccess: invalidate,
  })
  const restart = useMutation({
    mutationFn: () => restartApp(app.name),
    onSuccess: invalidate,
  })

  const pending = start.isPending || stop.isPending || restart.isPending

  return (
    <div className="space-y-4">
      <div className="flex gap-2">
        {app.status !== 'running' && app.docker_image && (
          <Button
            variant="outline"
            size="sm"
            onClick={() => start.mutate()}
            disabled={pending}
          >
            <Play className="mr-1 h-3 w-3" />
            Start
          </Button>
        )}
        {app.status === 'running' && (
          <>
            <Button
              variant="outline"
              size="sm"
              onClick={() => stop.mutate()}
              disabled={pending}
            >
              <Square className="mr-1 h-3 w-3" />
              Stop
            </Button>
            <Button
              variant="outline"
              size="sm"
              onClick={() => restart.mutate()}
              disabled={pending}
            >
              <RotateCw className="mr-1 h-3 w-3" />
              Restart
            </Button>
          </>
        )}
      </div>

      {status.data && (
        <div className="grid gap-3 sm:grid-cols-3">
          <MetricCard
            label="CPU"
            value={`${status.data.cpu_percent.toFixed(1)}%`}
            percent={status.data.cpu_percent}
          />
          <MetricCard
            label="Memory"
            value={`${status.data.memory_mb.toFixed(0)} / ${status.data.memory_limit_mb.toFixed(0)} MB`}
            percent={
              (status.data.memory_mb / status.data.memory_limit_mb) * 100
            }
          />
          <MetricCard label="Uptime" value={status.data.uptime} />
        </div>
      )}

      {app.status === 'running' && !status.data && status.isLoading && (
        <p className="text-sm text-muted-foreground">
          Loading container stats...
        </p>
      )}

      {!app.docker_image && (
        <p className="text-sm text-muted-foreground">
          No image built yet. Push code to the git remote or trigger a deploy.
        </p>
      )}
    </div>
  )
}

function MetricCard({
  label,
  value,
  percent,
}: {
  label: string
  value: string
  percent?: number
}) {
  return (
    <Card>
      <CardContent className="pt-4">
        <p className="text-xs text-muted-foreground">{label}</p>
        <p className="mt-1 font-mono text-sm">{value}</p>
        {percent !== undefined && (
          <div className="mt-2 h-1.5 overflow-hidden rounded-full bg-secondary">
            <div
              className={`h-full rounded-full transition-all ${
                percent > 80
                  ? 'bg-destructive'
                  : percent > 50
                    ? 'bg-chart-5'
                    : 'bg-chart-2'
              }`}
              style={{ width: `${Math.min(100, percent)}%` }}
            />
          </div>
        )}
      </CardContent>
    </Card>
  )
}
