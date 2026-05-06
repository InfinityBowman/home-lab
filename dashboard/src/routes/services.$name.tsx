import { useState } from 'react'
import { createFileRoute, useNavigate } from '@tanstack/react-router'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { getService, deleteService } from '@/lib/api'
import { Button } from '@/components/ui/button'
import { KeyRound } from 'lucide-react'
import { ServiceSecretsPanel } from '@/features/services/ServiceSecretsPanel'

export const Route = createFileRoute('/services/$name')({
  component: ServiceDetail,
})

function ServiceDetail() {
  const { name } = Route.useParams()
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const [confirmDelete, setConfirmDelete] = useState(false)

  const service = useQuery({
    queryKey: ['service', name],
    queryFn: () => getService(name),
  })

  const remove = useMutation({
    mutationFn: () => deleteService(name),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['services'] })
      navigate({ to: '/services' })
    },
  })

  if (service.isLoading)
    return <p className="text-sm text-muted-foreground">Loading...</p>
  if (service.error)
    return <p className="text-sm text-destructive">Service not found</p>
  if (!service.data) return null

  return (
    <div className="space-y-6">
      <div className="flex items-start justify-between">
        <div>
          <div className="flex items-center gap-2">
            <KeyRound className="h-5 w-5 text-muted-foreground" />
            <h2 className="font-mono text-xl font-bold">{service.data.name}</h2>
          </div>
          <p className="mt-1 text-xs text-muted-foreground/60">
            {service.data.compose_path}
          </p>
        </div>
        <div>
          {!confirmDelete ? (
            <Button
              variant="ghost"
              size="sm"
              onClick={() => setConfirmDelete(true)}
              className="text-muted-foreground hover:text-destructive"
            >
              Unregister
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
                Confirm
              </Button>
            </div>
          )}
        </div>
      </div>

      <div>
        <h3 className="mb-3 text-sm font-medium">Secrets</h3>
        <ServiceSecretsPanel serviceName={name} />
      </div>
    </div>
  )
}
