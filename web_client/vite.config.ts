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
            },
            '/ws_handler': {
                target: 'ws://localhost:3000',
                ws: true,
            }
        }
    }
}))
