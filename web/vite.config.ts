import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import wasm from 'vite-plugin-wasm'
import topLevelAwait from 'vite-plugin-top-level-await'

const honoPort = process.env.PORT || 3000

export default defineConfig({
  plugins: [react(), wasm(), topLevelAwait()],
  server: {
    host: true,
    proxy: {
      // Auth routes → Hono server
      '/auth': {
        target: `http://localhost:${honoPort}`,
        changeOrigin: true,
      },
    },
  },
})
