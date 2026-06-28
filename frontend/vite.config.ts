import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import path from 'path';

// https://vite.dev/config/
export default defineConfig({
   plugins: [react()],
   resolve: {
      alias: {
         '@': path.resolve(__dirname, './src'),
      },
   },
   server: {
      host: '127.0.0.1',
      port: 3001,
      strictPort: false,
      proxy: {
         '/api': {
            target: 'http://localhost:8086',
            changeOrigin: true,
            // rewrite: (path) => path.replace(/^\/api/, ''), // 【按需开启】如果后端接口本身没有 /api 前缀，把这行注释打开
         },
      },
   },
});
