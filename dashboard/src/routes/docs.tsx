import { createFileRoute } from '@tanstack/react-router'
import { Badge } from '@/components/ui/badge'

export const Route = createFileRoute('/docs')({
  component: Docs,
})

function Docs() {
  return (
    <div className="mx-auto max-w-4xl pb-16">
      <h1 className="text-2xl font-bold">Documentation</h1>
      <p className="mt-1 text-sm text-muted-foreground">
        How the HomeLab PaaS works end-to-end.
      </p>

      <Section title="Overview">
        <p>
          A self-hosted mini-PaaS running on an HP laptop (Ubuntu Server 24.04)
          at <Code>192.168.1.100</Code>. Push code via git, it builds a Docker
          image, deploys it as a container, and makes it accessible via
          Cloudflare Tunnel at <Code>*.jacobmaynard.dev</Code>.
        </p>
      </Section>

      <Section title="Traffic Flow">
        <CodeBlock>
{`Internet
  │
  ▼
Cloudflare Edge (DDoS, SSL termination)
  │
  ▼
cloudflared (tunnel connector)
  │
  ▼
Traefik (reverse proxy, routes by Host header)
  │
  ▼
App Container (e.g. homelab-my-app)`}
        </CodeBlock>
        <p className="mt-3">
          Wildcard DNS (<Code>*.jacobmaynard.dev</Code>) points to the tunnel.
          Traefik auto-discovers containers via Docker labels. No per-service
          DNS or tunnel config needed.
        </p>
      </Section>

      <Section title="Creating an App">
        <Steps>
          <Step n={1} title="Create via Dashboard or API">
            <p>
              Use the <strong>Create App</strong> page or{' '}
              <Code>POST /api/v1/apps</Code> with a name and port. This creates
              a SQLite record, a bare git repo, and a post-receive hook.
            </p>
            <p className="mt-2">
              The <strong>port</strong> must match what your app listens on
              inside the container (e.g. 3000 for Node, 8080 for Go). Traefik
              uses it to route traffic:{' '}
              <Code>my-app.jacobmaynard.dev</Code> →{' '}
              <Code>homelab-my-app:3000</Code>. Multiple apps can use the same
              port number — each container has its own network namespace, so
              there are no conflicts. Traefik routes by hostname, not port.
            </p>
          </Step>
          <Step n={2} title="Add the Git Remote">
            <CodeBlock>
              git remote add deploy
              ssh://paas@192.168.1.100/git-repos/my-app.git
            </CodeBlock>
          </Step>
          <Step n={3} title="Set Environment Variables (optional)">
            <p>
              Use the <strong>Env Vars</strong> tab on the app detail page, or{' '}
              <Code>PUT /api/v1/apps/my-app/env</Code>.
            </p>
          </Step>
          <Step n={4} title="Push to Deploy">
            <CodeBlock>git push deploy main</CodeBlock>
            <p className="mt-2">
              The post-receive hook triggers the build pipeline automatically.
            </p>
          </Step>
        </Steps>
      </Section>

      <Section title="Deploy Pipeline">
        <p className="mb-4">
          When you <Code>git push deploy main</Code>, the following happens:
        </p>
        <Steps>
          <Step n={1} title="Record Deployment">
            <p>
              A deployment record is created with status{' '}
              <Badge variant="secondary">pending</Badge>. The app status
              changes to <Badge variant="secondary">building</Badge>.
            </p>
          </Step>
          <Step n={2} title="Checkout Code">
            <p>
              The commit is checked out from the bare repo to a temp directory.
            </p>
          </Step>
          <Step n={3} title="Build Docker Image">
            <p>
              Docker builds the image from the app's <Code>Dockerfile</Code>,
              tagged as <Code>{'homelab/<app>:<sha>'}</Code>. Build output is
              streamed to the deployment log.
            </p>
          </Step>
          <Step n={4} title="Swap Container">
            <p>
              The old container is stopped and removed. A new container starts
              with the fresh image, env vars from the database, and Traefik
              labels for routing.
            </p>
          </Step>
          <Step n={5} title="Update Cloudflare">
            <p>
              Tunnel ingress rules and DNS records are synced so the app is
              publicly accessible.
            </p>
          </Step>
          <Step n={6} title="Finalize">
            <p>
              Deployment status → <Badge variant="default">succeeded</Badge>,
              app status → <Badge variant="default">running</Badge>. Temp build
              directory is cleaned up.
            </p>
          </Step>
        </Steps>
      </Section>

      <Section title="Rollback">
        <p>
          Every deployment tags the Docker image by commit SHA. To rollback,
          click <strong>Rollback</strong> on any previous succeeded deployment.
          This re-runs the container swap (steps 4–6) using the old image —{' '}
          <strong>no rebuild needed</strong>.
        </p>
      </Section>

      <Section title="App Requirements">
        <p>
          Each app needs a <Code>Dockerfile</Code> in its root. The PaaS
          doesn't care what language — if it has a Dockerfile, it can be
          deployed.
        </p>
        <CodeBlock>
{`FROM node:20-slim
WORKDIR /app
COPY package*.json ./
RUN npm ci --production
COPY . .
EXPOSE 3000
CMD ["node", "server.js"]`}
        </CodeBlock>
        <ul className="mt-4 space-y-1.5 text-muted-foreground">
          <li>
            Container name: <Code>homelab-{'<name>'}</Code>
          </li>
          <li>
            Domain: <Code>{'<name>'}.jacobmaynard.dev</Code>
          </li>
          <li>Restart policy: unless-stopped</li>
          <li>Network: homelab (shared Docker bridge)</li>
        </ul>
      </Section>

      <Section title="Managed Services">
        <p>
          Independent Docker Compose stacks in <Code>services/</Code>, deployed
          automatically when their files change on push to main.
        </p>
        <div className="mt-4 grid gap-px overflow-hidden rounded-lg border border-border bg-border sm:grid-cols-2">
          <ServiceCell name="n8n" desc="Workflow automation" />
          <ServiceCell name="plausible" desc="Privacy-first analytics" />
          <ServiceCell name="dozzle" desc="Real-time Docker log viewer" />
          <ServiceCell name="paleo-gateway" desc="Discord Gateway listener" />
        </div>
      </Section>

      <Section title="CI/CD Pipeline">
        <p>
          Push to <Code>main</Code> triggers GitHub Actions:
        </p>
        <Steps>
          <Step n={1} title="CI (GitHub-hosted)">
            <p>
              <Code>cargo fmt</Code>, <Code>clippy</Code>,{' '}
              <Code>cargo test</Code>, Docker build test for API and Dashboard.
            </p>
          </Step>
          <Step n={2} title="Deploy (self-hosted runner)">
            <p>
              The runner on the laptop pulls the latest code, rebuilds
              infrastructure containers, and restarts any services with changed
              files.
            </p>
          </Step>
        </Steps>
      </Section>

      <Section title="API Reference">
        <p>
          Base URL: <Code>/api/v1</Code>. All responses use the envelope:{' '}
          <Code>{'{ success, data?, error? }'}</Code>
        </p>
        <div className="mt-4 overflow-hidden rounded-lg border border-border">
          <Endpoint method="GET" path="/apps" desc="List all apps" />
          <Endpoint method="POST" path="/apps" desc="Create app" alt />
          <Endpoint method="GET" path="/apps/:name" desc="Get app details" />
          <Endpoint
            method="PUT"
            path="/apps/:name"
            desc="Update app"
            alt
          />
          <Endpoint method="DELETE" path="/apps/:name" desc="Delete app" />
          <Endpoint
            method="POST"
            path="/apps/:name/start"
            desc="Start container"
            alt
          />
          <Endpoint
            method="POST"
            path="/apps/:name/stop"
            desc="Stop container"
          />
          <Endpoint
            method="POST"
            path="/apps/:name/restart"
            desc="Restart container"
            alt
          />
          <Endpoint
            method="GET"
            path="/apps/:name/status"
            desc="Live container stats"
          />
          <Endpoint
            method="GET"
            path="/apps/:name/logs?tail=N"
            desc="Container logs"
            alt
          />
          <Endpoint
            method="POST"
            path="/apps/:name/deploy"
            desc="Trigger deploy"
          />
          <Endpoint
            method="GET"
            path="/apps/:name/deployments"
            desc="List deployments"
            alt
          />
          <Endpoint
            method="GET"
            path="/apps/:name/deployments/:id"
            desc="Deployment detail"
          />
          <Endpoint
            method="POST"
            path="/apps/:name/deployments/:id/rollback"
            desc="Rollback"
            alt
          />
          <Endpoint
            method="GET"
            path="/apps/:name/env"
            desc="List env vars (masked)"
          />
          <Endpoint
            method="PUT"
            path="/apps/:name/env"
            desc="Bulk set env vars"
            alt
          />
          <Endpoint
            method="DELETE"
            path="/apps/:name/env/:key"
            desc="Delete env var"
          />
          <Endpoint
            method="GET"
            path="/system/health"
            desc="Health check (public)"
            alt
          />
          <Endpoint method="GET" path="/system/info" desc="System info" />
        </div>
      </Section>

      <Section title="Tech Stack">
        <div className="flex flex-wrap gap-1.5">
          {[
            'Rust',
            'axum',
            'bollard',
            'sqlx',
            'SQLite',
            'Docker',
            'Traefik',
            'Cloudflare Tunnel',
            'React 19',
            'TanStack Router',
            'TanStack Query',
            'Tailwind v4',
            'shadcn/ui',
            'Terraform',
            'GitHub Actions',
          ].map((tech) => (
            <Badge key={tech} variant="outline" className="font-normal">
              {tech}
            </Badge>
          ))}
        </div>
      </Section>
    </div>
  )
}

function Section({
  title,
  children,
}: {
  title: string
  children: React.ReactNode
}) {
  return (
    <section className="mt-10">
      <h2 className="mb-4 text-lg font-semibold">{title}</h2>
      <div className="text-sm leading-relaxed text-foreground/90">
        {children}
      </div>
    </section>
  )
}

function Steps({ children }: { children: React.ReactNode }) {
  return <div className="space-y-4">{children}</div>
}

function Step({
  n,
  title,
  children,
}: {
  n: number
  title: string
  children?: React.ReactNode
}) {
  return (
    <div className="flex gap-3">
      <span className="flex h-6 w-6 shrink-0 items-center justify-center rounded-full bg-muted text-xs font-medium text-muted-foreground">
        {n}
      </span>
      <div className="min-w-0">
        <p className="font-medium">{title}</p>
        {children && (
          <div className="mt-1 text-muted-foreground">{children}</div>
        )}
      </div>
    </div>
  )
}

function Code({ children }: { children: React.ReactNode }) {
  return (
    <code className="rounded bg-muted px-1 py-0.5 font-mono text-xs">
      {children}
    </code>
  )
}

function CodeBlock({ children }: { children: React.ReactNode }) {
  return (
    <pre className="mt-3 overflow-x-auto rounded-lg bg-muted/50 p-3 font-mono text-xs leading-relaxed">
      {children}
    </pre>
  )
}

function ServiceCell({ name, desc }: { name: string; desc: string }) {
  return (
    <div className="bg-card px-4 py-3">
      <p className="font-mono text-sm font-medium">{name}</p>
      <p className="text-xs text-muted-foreground">{desc}</p>
    </div>
  )
}

function Endpoint({
  method,
  path,
  desc,
  alt,
}: {
  method: string
  path: string
  desc: string
  alt?: boolean
}) {
  const methodColor =
    method === 'GET'
      ? 'text-emerald-400'
      : method === 'POST'
        ? 'text-blue-400'
        : method === 'PUT'
          ? 'text-amber-400'
          : 'text-red-400'

  return (
    <div
      className={`flex items-baseline gap-2 px-3 py-1.5 font-mono text-xs ${alt ? 'bg-muted/30' : ''}`}
    >
      <span className={`w-12 shrink-0 font-semibold ${methodColor}`}>
        {method}
      </span>
      <span className="min-w-0 truncate">{path}</span>
      <span className="ml-auto shrink-0 text-muted-foreground">{desc}</span>
    </div>
  )
}
