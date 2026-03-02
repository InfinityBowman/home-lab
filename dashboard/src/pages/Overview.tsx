import { useQuery } from "@tanstack/react-query";
import { listApps, getSystemInfo } from "@/lib/api";
import { AppCard } from "@/components/AppCard";

export function Overview() {
  const apps = useQuery({ queryKey: ["apps"], queryFn: listApps, refetchInterval: 10_000 });
  const system = useQuery({ queryKey: ["system-info"], queryFn: getSystemInfo, refetchInterval: 30_000 });

  return (
    <div>
      {/* System health bar */}
      <div className="mb-6 flex items-center gap-6 rounded-lg border border-neutral-800 bg-neutral-900 px-5 py-3 text-sm">
        {system.data ? (
          <>
            <Stat label="Apps" value={system.data.app_count} />
            <Stat label="Running" value={system.data.container_count} />
            <Stat label="Docker" value={system.data.docker_version} />
            <Stat label="API" value={`v${system.data.version}`} />
          </>
        ) : system.isLoading ? (
          <span className="text-neutral-500">Loading...</span>
        ) : (
          <span className="text-red-400">Failed to connect to API</span>
        )}
      </div>

      {/* App grid */}
      <h2 className="mb-3 text-lg font-semibold">Apps</h2>
      {apps.isLoading && <p className="text-sm text-neutral-500">Loading...</p>}
      {apps.error && (
        <p className="text-sm text-red-400">Failed to load apps: {(apps.error as Error).message}</p>
      )}
      {apps.data && apps.data.length === 0 && (
        <p className="text-sm text-neutral-500">No apps yet. Create one to get started.</p>
      )}
      {apps.data && apps.data.length > 0 && (
        <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
          {apps.data.map((app) => (
            <AppCard key={app.id} app={app} />
          ))}
        </div>
      )}
    </div>
  );
}

function Stat({ label, value }: { label: string; value: string | number }) {
  return (
    <div>
      <span className="text-neutral-500">{label}</span>{" "}
      <span className="font-mono text-neutral-200">{value}</span>
    </div>
  );
}
