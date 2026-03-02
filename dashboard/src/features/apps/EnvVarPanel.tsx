import { useState } from 'react'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { listEnvVars, bulkSetEnvVars, deleteEnvVar } from '@/lib/api'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Card, CardContent } from '@/components/ui/card'
import { Trash2 } from 'lucide-react'

export function EnvVarPanel({ appName }: { appName: string }) {
  const queryClient = useQueryClient()
  const [newKey, setNewKey] = useState('')
  const [newValue, setNewValue] = useState('')

  const envVars = useQuery({
    queryKey: ['env', appName],
    queryFn: () => listEnvVars(appName),
  })

  const addVar = useMutation({
    mutationFn: () => bulkSetEnvVars(appName, { [newKey]: newValue }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['env', appName] })
      setNewKey('')
      setNewValue('')
    },
  })

  const removeVar = useMutation({
    mutationFn: (key: string) => deleteEnvVar(appName, key),
    onSuccess: () =>
      queryClient.invalidateQueries({ queryKey: ['env', appName] }),
  })

  if (envVars.isLoading)
    return <p className="text-sm text-muted-foreground">Loading...</p>
  if (envVars.error)
    return (
      <p className="text-sm text-destructive">
        Failed to load env vars: {(envVars.error as Error).message}
      </p>
    )

  return (
    <div className="space-y-4">
      {envVars.data && envVars.data.length > 0 && (
        <div className="space-y-1">
          {envVars.data.map((v) => (
            <Card key={v.key}>
              <CardContent className="flex items-center justify-between py-2">
                <div className="flex gap-3 font-mono text-xs">
                  <span>{v.key}</span>
                  <span className="text-muted-foreground">{v.value}</span>
                </div>
                <Button
                  variant="ghost"
                  size="icon"
                  onClick={() => removeVar.mutate(v.key)}
                  disabled={removeVar.isPending}
                  className="h-7 w-7 text-muted-foreground hover:text-destructive"
                >
                  <Trash2 className="h-3 w-3" />
                </Button>
              </CardContent>
            </Card>
          ))}
        </div>
      )}

      {envVars.data?.length === 0 && (
        <p className="text-sm text-muted-foreground">
          No environment variables set.
        </p>
      )}

      <form
        onSubmit={(e) => {
          e.preventDefault()
          if (newKey) addVar.mutate()
        }}
        className="flex gap-2"
      >
        <Input
          value={newKey}
          onChange={(e) => setNewKey(e.target.value.toUpperCase())}
          placeholder="KEY"
          className="flex-1 font-mono text-xs"
        />
        <Input
          value={newValue}
          onChange={(e) => setNewValue(e.target.value)}
          placeholder="value"
          className="flex-1 font-mono text-xs"
        />
        <Button
          type="submit"
          variant="outline"
          size="sm"
          disabled={addVar.isPending || !newKey}
        >
          Add
        </Button>
      </form>

      {addVar.error && (
        <p className="text-xs text-destructive">
          {(addVar.error as Error).message}
        </p>
      )}
    </div>
  )
}
