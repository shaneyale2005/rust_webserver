import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'

// https://vite.dev/config/
export default defineConfig({
  plugins: [vue()],
  base: '/browser/',  // 设置基础路径，确保资源路径正确
  build: {
    outDir: '../static/browser',
    emptyOutDir: true,
  }
})
