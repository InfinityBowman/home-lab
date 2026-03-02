import { useState } from 'react'
import { createFileRoute, useNavigate } from '@tanstack/react-router'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { createApp } from '@/lib/api'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'

export const Route = createFileRoute('/apps/new')({
  component: CreateApp,
})

function CreateApp() {
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const [name, setName] = useState('')
  const [port, setPort] = useState('3000')

  const portNum = Number(port)
  const portValid =
    Number.isInteger(portNum) && portNum >= 1 && portNum <= 65535

  const mutation = useMutation({
    mutationFn: () => createApp({ name, port: portNum }),
    onSuccess: (app) => {
      queryClient.invalidateQueries({ queryKey: ['apps'] })
      navigate({ to: '/apps/$name', params: { name: app.name } })
    },
  })

  return (
    <div className="mx-auto max-w-md">
      <Card>
        <CardHeader>
          <CardTitle>Create App</CardTitle>
        </CardHeader>
        <CardContent>
          <form
            onSubmit={(e) => {
              e.preventDefault()
              mutation.mutate()
            }}
            className="space-y-4"
          >
            <div className="space-y-2">
              <Label htmlFor="name">App Name</Label>
              <Input
                id="name"
                value={name}
                onChange={(e) => setName(e.target.value)}
                placeholder="my-app"
                required
                pattern="[a-z][a-z0-9-]*[a-z0-9]"
                className="font-mono"
              />
              <p className="text-xs text-muted-foreground">
                Lowercase, alphanumeric and hyphens. Becomes{' '}
                <span className="font-mono">
                  {name || 'my-app'}.jacobmaynard.dev
                </span>
              </p>
            </div>
            <div className="space-y-2">
              <Label htmlFor="port">Port</Label>
              <Input
                id="port"
                type="number"
                value={port}
                onChange={(e) => setPort(e.target.value)}
                min={1}
                max={65535}
                className="font-mono"
              />
              <p className="text-xs text-muted-foreground">
                Internal container port your app listens on
              </p>
            </div>

            {mutation.error && (
              <p className="text-sm text-destructive">
                {(mutation.error as Error).message}
              </p>
            )}

            <Button
              type="submit"
              className="w-full"
              disabled={mutation.isPending || !name || !portValid}
            >
              {mutation.isPending ? 'Creating...' : 'Create App'}
            </Button>
          </form>
        </CardContent>
      </Card>
    </div>
  )
}
