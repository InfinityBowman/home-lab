import { useState } from 'react'
import { createFileRoute, useNavigate } from '@tanstack/react-router'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { getApp, deleteApp } from '@/lib/api'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { statusVariant } from '@/lib/utils'
import { AppStatusPanel } from '@/features/apps/AppStatusPanel'
import { AppLogsPanel } from '@/features/apps/AppLogsPanel'
import { DeployButton } from '@/features/apps/DeployButton'
import { DeploymentList } from '@/features/apps/DeploymentList'
import { EnvVarPanel } from '@/features/apps/EnvVarPanel'

export const Route = createFileRoute('/apps/$name')({
  component: AppDetail,
})

function AppDetail() {
  const { name } = Route.useParams()
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const [confirmDelete, setConfirmDelete] = useState(false)

  const app = useQuery({
    queryKey: ['app', name],
    queryFn: () => getApp(name),
    refetchInterval: 10_000,
  })

  const remove = useMutation({
    mutationFn: () => deleteApp(name),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['apps'] })
      navigate({ to: '/' })
    },
  })

  if (app.isLoading)
    return <p className="text-sm text-muted-foreground">Loading...</p>
  if (app.error)
    return <p className="text-sm text-destructive">App not found</p>
  if (!app.data) return null

  const a = app.data

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-start justify-between">
        <div>
          <div className="flex items-center gap-3">
            <h2 className="font-mono text-xl font-bold">{a.name}</h2>
            <Badge variant={statusVariant(a.status)}>{a.status}</Badge>
          </div>
          <p className="mt-1 text-sm text-muted-foreground">{a.domain}</p>
          <p className="text-xs text-muted-foreground/60">
            Port {a.port}
            {a.docker_image && <span className="ml-2">{a.docker_image}</span>}
          </p>
        </div>
        <div className="flex items-center gap-2">
          <DeployButton appName={a.name} />
          {!confirmDelete ? (
            <Button
              variant="ghost"
              size="sm"
              onClick={() => setConfirmDelete(true)}
              className="text-muted-foreground hover:text-destructive"
            >
              Delete
            </Button>
          ) : (
            <div className="flex gap-1">
              <Button
                variant="ghost"
                size="sm"
                onClick={() => setConfirmDelete(false)}
              >
                Cancel
              </Button>
              <Button
                variant="destructive"
                size="sm"
                onClick={() => remove.mutate()}
                disabled={remove.isPending}
              >
                Confirm Delete
              </Button>
            </div>
          )}
        </div>
      </div>

      {/* Tabs */}
      <Tabs defaultValue="status">
        <TabsList>
          <TabsTrigger value="status">Status</TabsTrigger>
          <TabsTrigger value="logs">Logs</TabsTrigger>
          <TabsTrigger value="deployments">Deployments</TabsTrigger>
          <TabsTrigger value="env">Env Vars</TabsTrigger>
        </TabsList>
        <TabsContent value="status" className="mt-4">
          <AppStatusPanel app={a} />
        </TabsContent>
        <TabsContent value="logs" className="mt-4">
          <AppLogsPanel appName={a.name} enabled={a.status === 'running'} />
        </TabsContent>
        <TabsContent value="deployments" className="mt-4">
          <DeploymentList appName={a.name} />
        </TabsContent>
        <TabsContent value="env" className="mt-4">
          <EnvVarPanel appName={a.name} />
        </TabsContent>
      </Tabs>
    </div>
  )
}
