import type { AppStatus, DeployStatus } from "@/types/api";

export function statusColor(status: AppStatus | DeployStatus): string {
  switch (status) {
    case "running":
    case "succeeded":
      return "text-emerald-400";
    case "building":
    case "deploying":
    case "pending":
      return "text-amber-400";
    case "stopped":
    case "created":
      return "text-neutral-400";
    case "failed":
      return "text-red-400";
    default: {
      const _exhaustive: never = status;
      void _exhaustive;
      return "text-neutral-400";
    }
  }
}

export function statusDot(status: AppStatus): string {
  switch (status) {
    case "running":
      return "bg-emerald-400";
    case "building":
      return "bg-amber-400";
    case "stopped":
    case "created":
      return "bg-neutral-500";
    case "failed":
      return "bg-red-400";
    default: {
      const _exhaustive: never = status;
      void _exhaustive;
      return "bg-neutral-500";
    }
  }
}

export function timeAgo(iso: string): string {
  const seconds = Math.floor((Date.now() - new Date(iso).getTime()) / 1000);
  if (seconds < 60) return "just now";
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.floor(hours / 24);
  return `${days}d ago`;
}

export function shortSha(sha: string): string {
  return sha.slice(0, 8);
}
