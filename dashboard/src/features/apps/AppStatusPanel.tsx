import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { getContainerStatus, startApp, stopApp, restartApp } from "@/lib/api";
import type { App } from "@/types/api";

export function AppStatusPanel({ app }: { app: App }) {
  const queryClient = useQueryClient();
  const invalidate = () => {
    queryClient.invalidateQueries({ queryKey: ["app", app.name] });
    queryClient.invalidateQueries({ queryKey: ["container-status", app.name] });
  };

  const status = useQuery({
    queryKey: ["container-status", app.name],
    queryFn: () => getContainerStatus(app.name),
    refetchInterval: 5_000,
    enabled: app.status === "running",
    retry: false,
  });

  const start = useMutation({ mutationFn: () => startApp(app.name), onSuccess: invalidate });
  const stop = useMutation({ mutationFn: () => stopApp(app.name), onSuccess: invalidate });
  const restart = useMutation({ mutationFn: () => restartApp(app.name), onSuccess: invalidate });

  const pending = start.isPending || stop.isPending || restart.isPending;

  return (
    <div className="space-y-4">
      {/* Action buttons */}
      <div className="flex gap-2">
        {app.status !== "running" && app.docker_image && (
          <ActionBtn onClick={() => start.mutate()} disabled={pending} label="Start" />
        )}
        {app.status === "running" && (
          <>
            <ActionBtn onClick={() => stop.mutate()} disabled={pending} label="Stop" />
            <ActionBtn onClick={() => restart.mutate()} disabled={pending} label="Restart" />
          </>
        )}
      </div>

      {/* Live stats */}
      {status.data && (
        <div className="grid gap-3 sm:grid-cols-3">
          <MetricCard label="CPU" value={`${status.data.cpu_percent.toFixed(1)}%`} percent={status.data.cpu_percent} />
          <MetricCard
            label="Memory"
            value={`${status.data.memory_mb.toFixed(0)} / ${status.data.memory_limit_mb.toFixed(0)} MB`}
            percent={(status.data.memory_mb / status.data.memory_limit_mb) * 100}
          />
          <MetricCard label="Uptime" value={status.data.uptime} />
        </div>
      )}

      {app.status === "running" && !status.data && status.isLoading && (
        <p className="text-sm text-neutral-500">Loading container stats...</p>
      )}

      {!app.docker_image && (
        <p className="text-sm text-neutral-500">
          No image built yet. Push code to the git remote or trigger a deploy.
        </p>
      )}
    </div>
  );
}

function ActionBtn({ onClick, disabled, label }: { onClick: () => void; disabled: boolean; label: string }) {
  return (
    <button
      onClick={onClick}
      disabled={disabled}
      className="rounded-md border border-neutral-700 px-3 py-1.5 text-xs font-medium text-neutral-300 transition hover:border-neutral-500 hover:text-neutral-100 disabled:opacity-50"
    >
      {label}
    </button>
  );
}

function MetricCard({ label, value, percent }: { label: string; value: string; percent?: number }) {
  return (
    <div className="rounded-lg border border-neutral-800 bg-neutral-900 p-3">
      <p className="text-xs text-neutral-500">{label}</p>
      <p className="mt-1 font-mono text-sm text-neutral-100">{value}</p>
      {percent !== undefined && (
        <div className="mt-2 h-1.5 overflow-hidden rounded-full bg-neutral-800">
          <div
            className={`h-full rounded-full transition-all ${percent > 80 ? "bg-red-400" : percent > 50 ? "bg-amber-400" : "bg-emerald-400"}`}
            style={{ width: `${Math.min(100, percent)}%` }}
          />
        </div>
      )}
    </div>
  );
}
