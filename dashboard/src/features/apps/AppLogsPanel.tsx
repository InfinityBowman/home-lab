import { useQuery } from "@tanstack/react-query";
import { getContainerLogs } from "@/lib/api";
import { useRef, useEffect } from "react";

export function AppLogsPanel({ appName, enabled }: { appName: string; enabled: boolean }) {
  const bottomRef = useRef<HTMLDivElement>(null);

  const logs = useQuery({
    queryKey: ["logs", appName],
    queryFn: () => getContainerLogs(appName, 500),
    refetchInterval: 5_000,
    enabled,
  });

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [logs.data]);

  if (!enabled) {
    return <p className="text-sm text-neutral-500">Container is not running.</p>;
  }

  if (logs.isLoading) {
    return <p className="text-sm text-neutral-500">Loading logs...</p>;
  }

  if (logs.error) {
    return <p className="text-sm text-red-400">Failed to load logs: {(logs.error as Error).message}</p>;
  }

  if (!logs.data || logs.data.length === 0) {
    return <p className="text-sm text-neutral-500">No logs yet.</p>;
  }

  return (
    <div className="max-h-96 overflow-auto rounded-lg border border-neutral-800 bg-neutral-950 p-3 font-mono text-xs leading-relaxed text-neutral-300">
      {logs.data.map((line, i) => (
        <div key={i} className="whitespace-pre-wrap break-all">
          {line}
        </div>
      ))}
      <div ref={bottomRef} />
    </div>
  );
}
