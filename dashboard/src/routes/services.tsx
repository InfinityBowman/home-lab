import { useState } from 'react'
import { createFileRoute, Link } from '@tanstack/react-router'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { listServices, createService } from '@/lib/api'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { KeyRound } from 'lucide-react'
import { timeAgo } from '@/lib/utils'

export const Route = createFileRoute('/services')({
  component: Services,
})

function Services() {
  const queryClient = useQueryClient()
  const [newName, setNewName] = useState('')

  const services = useQuery({
    queryKey: ['services'],
    queryFn: listServices,
  })

  const add = useMutation({
    mutationFn: () => createService({ name: newName }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['services'] })
      setNewName('')
    },
  })

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-lg font-semibold">Services</h2>
        <p className="text-sm text-muted-foreground">
          Manage secrets for docker-compose services
        </p>
      </div>

      {services.isLoading && (
        <p className="text-sm text-muted-foreground">Loading...</p>
      )}
      {services.error && (
        <p className="text-sm text-destructive">
          Failed to load services: {(services.error as Error).message}
        </p>
      )}

      {services.data && services.data.length > 0 && (
        <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
          {services.data.map((svc) => (
            <Link
              key={svc.id}
              to="/services/$name"
              params={{ name: svc.name }}
              className="block"
            >
              <Card className="transition hover:border-ring/30">
                <CardHeader className="flex flex-row items-center gap-2 pb-2">
                  <KeyRound className="h-4 w-4 text-muted-foreground" />
                  <CardTitle className="font-mono text-sm">
                    {svc.name}
                  </CardTitle>
                </CardHeader>
                <CardContent className="text-xs text-muted-foreground">
                  <p className="opacity-60">
                    Added {timeAgo(svc.created_at)}
                  </p>
                </CardContent>
              </Card>
            </Link>
          ))}
        </div>
      )}

      {services.data?.length === 0 && (
        <p className="text-sm text-muted-foreground">
          No services registered. Add one below.
        </p>
      )}

      <Card>
        <CardHeader>
          <CardTitle className="text-sm">Register Service</CardTitle>
        </CardHeader>
        <CardContent>
          <form
            onSubmit={(e) => {
              e.preventDefault()
              if (newName) add.mutate()
            }}
            className="flex gap-2"
          >
            <Input
              value={newName}
              onChange={(e) => setNewName(e.target.value.toLowerCase())}
              placeholder="service name (e.g. n8n)"
              className="flex-1 font-mono text-sm"
            />
            <Button
              type="submit"
              variant="outline"
              size="sm"
              disabled={add.isPending || !newName}
            >
              Add
            </Button>
          </form>
          {add.error && (
            <p className="mt-2 text-xs text-destructive">
              {(add.error as Error).message}
            </p>
          )}
        </CardContent>
      </Card>
    </div>
  )
}
