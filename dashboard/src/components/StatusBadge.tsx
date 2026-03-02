import type { AppStatus, DeployStatus } from "@/types/api";
import { statusColor } from "@/lib/utils";

export function StatusBadge({ status }: { status: AppStatus | DeployStatus }) {
  return (
    <span
      className={`inline-flex items-center rounded-full border border-neutral-700 px-2.5 py-0.5 text-xs font-medium ${statusColor(status)}`}
    >
      {status}
    </span>
  );
}
