import { defineConfig, loadEnv } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'
import viteTsConfigPaths from 'vite-tsconfig-paths'
import { TanStackRouterVite } from '@tanstack/router-plugin/vite'

export default defineConfig(({ mode }) => {
  const env = loadEnv(mode, process.cwd(), '')

  return {
    plugins: [
      viteTsConfigPaths({ projects: ['./tsconfig.json'] }),
      tailwindcss(),
      TanStackRouterVite({ autoCodeSplitting: true }),
      react(),
    ],
    server: {
      proxy: {
        '/api': {
          target: env.API_URL || 'http://192.168.1.100:5170',
          changeOrigin: true,
          headers: env.API_SECRET
            ? { Authorization: `Bearer ${env.API_SECRET}` }
            : undefined,
        },
      },
    },
  }
})
