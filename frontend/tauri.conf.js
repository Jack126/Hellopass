const { defineConfig } = require('vite')
const vue = require('@vitejs/plugin-vue')
const path = require('path')

// https://vitejs.dev/config/
module.exports = defineConfig({
  plugins: [vue()],
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
  server: {
    port: 3000,
    strictPort: true,
    watch: {
      usePolling: true,
    },
  },
  build: {
    outDir: '../dist',
    assetsDir: 'assets',
    rollupOptions: {
      output: {
        manualChunks: {
          'vendor': ['vue', 'element-plus'],
          'utils': ['@vueuse/core'],
        },
      },
    },
  },
  define: {
    'process.env': {},
  },
})