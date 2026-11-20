import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'

// Tauri 2 expects a fixed dev server port and does not clear the screen.
export default defineConfig({
  plugins: [vue()],
  clearScreen: false,
  server: {
    host: '127.0.0.1',
    port: 5173,
    strictPort: true,
  },
  envPrefix: ['VITE_', 'TAURI_'],
  build: {
    target: 'es2022',
    outDir: 'dist',
    sourcemap: false,
  },
})
