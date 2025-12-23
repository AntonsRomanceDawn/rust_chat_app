import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

// https://vitejs.dev/config/
export default defineConfig(({ mode }) => ({
    plugins: [react()],
    esbuild: {
        drop: mode === 'production' ? ['console', 'debugger'] : [],
    },
    server: {
        proxy: {
            '/api': {
                target: 'http://localhost:3000',
                changeOrigin: true,
                rewrite: (path) => path.replace(/^\/api/, ''),
            },
            '/ws': {
                target: 'ws://localhost:3000',
                ws: true,
                rewrite: (path) => path.replace(/^\/ws/, ''),
            }
        }
    }
}))
