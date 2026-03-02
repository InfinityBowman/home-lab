import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { listDeployments, rollbackDeployment } from '@/lib/api'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card, CardContent } from '@/components/ui/card'
import { statusVariant, shortSha, timeAgo, durationStr } from '@/lib/utils'
import type { DeployStatus } from '@/types/api'

export function DeploymentList({ appName }: { appName: string }) {
  const queryClient = useQueryClient()

  const deployments = useQuery({
    queryKey: ['deployments', appName],
    queryFn: () => listDeployments(appName),
    refetchInterval: (query) => {
      const data = query.state.data
      const hasActive = data?.some((d) =>
        (['pending', 'building', 'deploying'] as DeployStatus[]).includes(
          d.status,
        ),
      )
      return hasActive ? 3_000 : 30_000
    },
  })

  const rollback = useMutation({
    mutationFn: (deploymentId: string) =>
      rollbackDeployment(appName, deploymentId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['deployments', appName] })
      queryClient.invalidateQueries({ queryKey: ['app', appName] })
    },
  })

  if (deployments.isLoading) {
    return <p className="text-sm text-muted-foreground">Loading...</p>
  }

  if (!deployments.data || deployments.data.length === 0) {
    return <p className="text-sm text-muted-foreground">No deployments yet.</p>
  }

  const latestSucceededId = deployments.data.find(
    (d) => d.status === 'succeeded',
  )?.id

  return (
    <div className="space-y-2">
      {deployments.data.map((d) => (
        <Card key={d.id}>
          <CardContent className="flex items-center justify-between py-3">
            <div className="flex items-center gap-3">
              <Badge variant={statusVariant(d.status)}>{d.status}</Badge>
              <span className="font-mono text-xs">{shortSha(d.commit_sha)}</span>
              <span className="text-xs text-muted-foreground">
                {timeAgo(d.started_at)}
              </span>
              {d.finished_at && (
                <span className="text-xs text-muted-foreground/60">
                  {durationStr(d.started_at, d.finished_at)}
                </span>
              )}
            </div>
            {d.status === 'succeeded' && d.id !== latestSucceededId && (
              <Button
                variant="ghost"
                size="sm"
                onClick={() => rollback.mutate(d.id)}
                disabled={rollback.isPending}
                className="text-muted-foreground"
              >
                Rollback
              </Button>
            )}
            {isActive(d.status) && (
              <span className="animate-pulse text-xs text-muted-foreground">
                In progress...
              </span>
            )}
          </CardContent>
        </Card>
      ))}
    </div>
  )
}

function isActive(status: DeployStatus) {
  return (['pending', 'building', 'deploying'] as DeployStatus[]).includes(
    status,
  )
}
