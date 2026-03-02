import { useMutation, useQueryClient } from "@tanstack/react-query";
import { triggerDeploy } from "@/lib/api";

export function DeployButton({ appName }: { appName: string }) {
  const queryClient = useQueryClient();

  const deploy = useMutation({
    mutationFn: () => triggerDeploy(appName),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["app", appName] });
      queryClient.invalidateQueries({ queryKey: ["deployments", appName] });
    },
  });

  return (
    <div className="flex items-center gap-3">
      <button
        onClick={() => deploy.mutate()}
        disabled={deploy.isPending}
        className="rounded-md bg-neutral-100 px-3 py-1.5 text-xs font-medium text-neutral-900 transition hover:bg-neutral-200 disabled:opacity-50"
      >
        {deploy.isPending ? "Deploying..." : "Deploy"}
      </button>
      {deploy.error && (
        <span className="text-xs text-red-400">{(deploy.error as Error).message}</span>
      )}
    </div>
  );
}
