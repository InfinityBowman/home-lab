import { useMutation, useQueryClient } from '@tanstack/react-query'
import { triggerDeploy } from '@/lib/api'
import { Button } from '@/components/ui/button'
import { Rocket } from 'lucide-react'

export function DeployButton({ appName }: { appName: string }) {
  const queryClient = useQueryClient()

  const deploy = useMutation({
    mutationFn: () => triggerDeploy(appName),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['app', appName] })
      queryClient.invalidateQueries({ queryKey: ['deployments', appName] })
    },
  })

  return (
    <div className="flex items-center gap-3">
      <Button
        size="sm"
        onClick={() => deploy.mutate()}
        disabled={deploy.isPending}
      >
        <Rocket className="mr-1 h-3 w-3" />
        {deploy.isPending ? 'Deploying...' : 'Deploy'}
      </Button>
      {deploy.error && (
        <span className="text-xs text-destructive">
          {(deploy.error as Error).message}
        </span>
      )}
    </div>
  )
}
