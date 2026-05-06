import { useState } from 'react'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import {
  listServiceSecrets,
  bulkSetServiceSecrets,
  deleteServiceSecret,
  revealServiceSecret,
} from '@/lib/api'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Card, CardContent } from '@/components/ui/card'
import { Trash2, Eye, EyeOff } from 'lucide-react'

export function ServiceSecretsPanel({
  serviceName,
}: {
  serviceName: string
}) {
  const queryClient = useQueryClient()
  const [newKey, setNewKey] = useState('')
  const [newValue, setNewValue] = useState('')
  const [revealed, setRevealed] = useState<Record<string, string>>({})

  const secrets = useQuery({
    queryKey: ['service-secrets', serviceName],
    queryFn: () => listServiceSecrets(serviceName),
  })

  const addSecret = useMutation({
    mutationFn: () =>
      bulkSetServiceSecrets(serviceName, { [newKey]: newValue }),
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: ['service-secrets', serviceName],
      })
      setNewKey('')
      setNewValue('')
    },
  })

  const removeSecret = useMutation({
    mutationFn: (key: string) => deleteServiceSecret(serviceName, key),
    onSuccess: () =>
      queryClient.invalidateQueries({
        queryKey: ['service-secrets', serviceName],
      }),
  })

  const reveal = useMutation({
    mutationFn: (key: string) => revealServiceSecret(serviceName, key),
    onSuccess: (data) => {
      setRevealed((prev) => ({ ...prev, [data.key]: data.value }))
      setTimeout(() => {
        setRevealed((prev) => {
          const next = { ...prev }
          delete next[data.key]
          return next
        })
      }, 10_000)
    },
  })

  if (secrets.isLoading)
    return <p className="text-sm text-muted-foreground">Loading...</p>
  if (secrets.error)
    return (
      <p className="text-sm text-destructive">
        Failed to load secrets: {(secrets.error as Error).message}
      </p>
    )

  return (
    <div className="space-y-4">
      {secrets.data && secrets.data.length > 0 && (
        <div className="space-y-1">
          {secrets.data.map((s) => (
            <Card key={s.key}>
              <CardContent className="flex items-center justify-between py-2">
                <div className="flex gap-3 font-mono text-xs">
                  <span>{s.key}</span>
                  <span className="text-muted-foreground">
                    {revealed[s.key] ?? s.value}
                  </span>
                </div>
                <div className="flex gap-1">
                  <Button
                    variant="ghost"
                    size="icon"
                    onClick={() => {
                      if (revealed[s.key]) {
                        setRevealed((prev) => {
                          const next = { ...prev }
                          delete next[s.key]
                          return next
                        })
                      } else {
                        reveal.mutate(s.key)
                      }
                    }}
                    disabled={reveal.isPending}
                    className="h-7 w-7 text-muted-foreground"
                  >
                    {revealed[s.key] ? (
                      <EyeOff className="h-3 w-3" />
                    ) : (
                      <Eye className="h-3 w-3" />
                    )}
                  </Button>
                  <Button
                    variant="ghost"
                    size="icon"
                    onClick={() => removeSecret.mutate(s.key)}
                    disabled={removeSecret.isPending}
                    className="h-7 w-7 text-muted-foreground hover:text-destructive"
                  >
                    <Trash2 className="h-3 w-3" />
                  </Button>
                </div>
              </CardContent>
            </Card>
          ))}
        </div>
      )}

      {secrets.data?.length === 0 && (
        <p className="text-sm text-muted-foreground">No secrets set.</p>
      )}

      <form
        onSubmit={(e) => {
          e.preventDefault()
          if (newKey) addSecret.mutate()
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
          disabled={addSecret.isPending || !newKey}
        >
          Add
        </Button>
      </form>

      {addSecret.error && (
        <p className="text-xs text-destructive">
          {(addSecret.error as Error).message}
        </p>
      )}
    </div>
  )
}
