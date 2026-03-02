import { Link } from "react-router-dom";
import type { App } from "@/types/api";
import { StatusBadge } from "@/components/StatusBadge";
import { statusDot, timeAgo } from "@/lib/utils";

export function AppCard({ app }: { app: App }) {
  return (
    <Link
      to={`/apps/${app.name}`}
      className="block rounded-lg border border-neutral-800 bg-neutral-900 p-4 transition hover:border-neutral-600"
    >
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <span className={`inline-block h-2 w-2 rounded-full ${statusDot(app.status)}`} />
          <h3 className="font-mono text-sm font-semibold text-neutral-100">{app.name}</h3>
        </div>
        <StatusBadge status={app.status} />
      </div>
      <div className="mt-3 space-y-1 text-xs text-neutral-400">
        <p>{app.domain}</p>
        <p>
          Port {app.port}
          {app.docker_image && (
            <span className="ml-2 text-neutral-500">{app.docker_image}</span>
          )}
        </p>
        <p className="text-neutral-500">Updated {timeAgo(app.updated_at)}</p>
      </div>
    </Link>
  );
}
