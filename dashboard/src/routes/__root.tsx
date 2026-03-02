import { Link, Outlet, createRootRoute } from '@tanstack/react-router'
import {
  Server,
  LayoutDashboard,
  PlusCircle,
  BookOpen,
} from 'lucide-react'

export const Route = createRootRoute({
  component: RootLayout,
})

const navItems = [
  { to: '/' as const, label: 'Overview', icon: LayoutDashboard },
  { to: '/apps/new' as const, label: 'Create App', icon: PlusCircle },
  { to: '/docs' as const, label: 'Docs', icon: BookOpen },
]

function RootLayout() {
  return (
    <div className="flex min-h-screen">
      <aside className="flex w-56 flex-col border-r border-border bg-sidebar px-3 py-5">
        <Link to="/" className="mb-8 flex items-center gap-2 px-2">
          <Server className="h-5 w-5 text-primary" />
          <span className="font-mono text-lg font-bold tracking-tight text-sidebar-foreground">
            HomeLab
          </span>
        </Link>
        <nav className="flex flex-col gap-1">
          {navItems.map((item) => (
            <Link
              key={item.to}
              to={item.to}
              activeOptions={{ exact: item.to === '/' }}
              className="flex items-center gap-2 rounded-md px-2 py-1.5 text-sm text-sidebar-foreground/60 transition hover:bg-sidebar-accent hover:text-sidebar-accent-foreground [&.active]:bg-sidebar-accent [&.active]:text-sidebar-accent-foreground"
            >
              <item.icon className="h-4 w-4" />
              {item.label}
            </Link>
          ))}
        </nav>
      </aside>
      <main className="flex-1 overflow-auto bg-background p-6">
        <Outlet />
      </main>
    </div>
  )
}
