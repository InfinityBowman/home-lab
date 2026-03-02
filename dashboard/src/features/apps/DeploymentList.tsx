import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { listDeployments, rollbackDeployment } from "@/lib/api";
import { StatusBadge } from "@/components/StatusBadge";
import { shortSha, timeAgo } from "@/lib/utils";
import type { DeployStatus } from "@/types/api";

export function DeploymentList({ appName }: { appName: string }) {
  const queryClient = useQueryClient();

  const deployments = useQuery({
    queryKey: ["deployments", appName],
    queryFn: () => listDeployments(appName),
    refetchInterval: (query) => {
      const data = query.state.data;
      const hasActive = data?.some((d) =>
        ["pending", "building", "deploying"].includes(d.status),
      );
      return hasActive ? 3_000 : 30_000;
    },
  });

  const rollback = useMutation({
    mutationFn: (deploymentId: string) => rollbackDeployment(appName, deploymentId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["deployments", appName] });
      queryClient.invalidateQueries({ queryKey: ["app", appName] });
    },
  });

  if (deployments.isLoading) {
    return <p className="text-sm text-neutral-500">Loading...</p>;
  }

  if (!deployments.data || deployments.data.length === 0) {
    return <p className="text-sm text-neutral-500">No deployments yet.</p>;
  }

  const latestSucceededId = deployments.data.find((d) => d.status === "succeeded")?.id;

  return (
    <div className="space-y-2">
      {deployments.data.map((d) => (
        <div
          key={d.id}
          className="flex items-center justify-between rounded-lg border border-neutral-800 bg-neutral-900 px-4 py-2.5"
        >
          <div className="flex items-center gap-3">
            <StatusBadge status={d.status} />
            <span className="font-mono text-xs text-neutral-300">{shortSha(d.commit_sha)}</span>
            <span className="text-xs text-neutral-500">{timeAgo(d.started_at)}</span>
            {d.finished_at && (
              <span className="text-xs text-neutral-600">
                {durationStr(d.started_at, d.finished_at)}
              </span>
            )}
          </div>
          {d.status === "succeeded" && d.id !== latestSucceededId && (
            <button
              onClick={() => rollback.mutate(d.id)}
              disabled={rollback.isPending}
              className="text-xs text-neutral-500 transition hover:text-neutral-200"
            >
              Rollback
            </button>
          )}
          {isActive(d.status) && (
            <span className="text-xs text-amber-400 animate-pulse">In progress...</span>
          )}
        </div>
      ))}
    </div>
  );
}

function isActive(status: DeployStatus) {
  return ["pending", "building", "deploying"].includes(status);
}

function durationStr(start: string, end: string) {
  const ms = new Date(end).getTime() - new Date(start).getTime();
  const secs = Math.floor(ms / 1000);
  if (secs < 60) return `${secs}s`;
  return `${Math.floor(secs / 60)}m ${secs % 60}s`;
}
