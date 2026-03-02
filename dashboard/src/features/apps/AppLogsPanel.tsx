import { useRef, useEffect } from 'react'
import { useQuery } from '@tanstack/react-query'
import { getContainerLogs } from '@/lib/api'

export function AppLogsPanel({
  appName,
  enabled,
}: {
  appName: string
  enabled: boolean
}) {
  const bottomRef = useRef<HTMLDivElement>(null)

  const logs = useQuery({
    queryKey: ['logs', appName],
    queryFn: () => getContainerLogs(appName, 500),
    refetchInterval: 5_000,
    enabled,
  })

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: 'smooth' })
  }, [logs.data])

  if (!enabled) {
    return (
      <p className="text-sm text-muted-foreground">Container is not running.</p>
    )
  }

  if (logs.isLoading) {
    return <p className="text-sm text-muted-foreground">Loading logs...</p>
  }

  if (logs.error) {
    return (
      <p className="text-sm text-destructive">
        Failed to load logs: {(logs.error as Error).message}
      </p>
    )
  }

  if (!logs.data || logs.data.length === 0) {
    return <p className="text-sm text-muted-foreground">No logs yet.</p>
  }

  return (
    <div className="max-h-96 overflow-auto rounded-lg border border-border bg-card p-3 font-mono text-xs leading-relaxed text-card-foreground">
      {logs.data.map((line, i) => (
        <div key={`${i}-${line.slice(0, 20)}`} className="whitespace-pre-wrap break-all">
          {line}
        </div>
      ))}
      <div ref={bottomRef} />
    </div>
  )
}
