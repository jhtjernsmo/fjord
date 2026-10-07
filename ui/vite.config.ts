import react from '@vitejs/plugin-react'
import { defineConfig } from 'vite'

// Tauri expects a fixed dev port.
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: { port: 5173, strictPort: true },
})
