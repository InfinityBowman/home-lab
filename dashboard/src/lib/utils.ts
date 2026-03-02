import { clsx, type ClassValue } from 'clsx'
import { twMerge } from 'tailwind-merge'
import type { AppStatus, DeployStatus } from '@/types/api'

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs))
}

export function statusVariant(
  status: AppStatus | DeployStatus,
): 'default' | 'secondary' | 'destructive' | 'outline' {
  switch (status) {
    case 'running':
    case 'succeeded':
      return 'default'
    case 'building':
    case 'deploying':
    case 'pending':
      return 'secondary'
    case 'stopped':
    case 'created':
      return 'outline'
    case 'failed':
      return 'destructive'
    default: {
      const _exhaustive: never = status
      void _exhaustive
      return 'outline'
    }
  }
}

export function statusDotColor(status: AppStatus): string {
  switch (status) {
    case 'running':
      return 'bg-emerald-500'
    case 'building':
      return 'bg-amber-500 animate-pulse'
    case 'stopped':
    case 'created':
      return 'bg-muted-foreground/40'
    case 'failed':
      return 'bg-destructive'
    default: {
      const _exhaustive: never = status
      void _exhaustive
      return 'bg-muted-foreground/40'
    }
  }
}

export function timeAgo(iso: string): string {
  const seconds = Math.floor((Date.now() - new Date(iso).getTime()) / 1000)
  if (seconds < 60) return 'just now'
  const minutes = Math.floor(seconds / 60)
  if (minutes < 60) return `${minutes}m ago`
  const hours = Math.floor(minutes / 60)
  if (hours < 24) return `${hours}h ago`
  const days = Math.floor(hours / 24)
  return `${days}d ago`
}

export function shortSha(sha: string): string {
  return sha.slice(0, 8)
}

export function durationStr(start: string, end: string): string {
  const ms = new Date(end).getTime() - new Date(start).getTime()
  const secs = Math.floor(ms / 1000)
  if (secs < 60) return `${secs}s`
  return `${Math.floor(secs / 60)}m ${secs % 60}s`
}
