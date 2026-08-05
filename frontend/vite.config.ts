/// <reference types="vitest/config" />
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
      port: 5173,
      strictPort: false,
      proxy: {
         // SSE 端点 — 优先匹配，禁用缓冲确保事件实时推送
         '/api/chat/stream': {
            target: 'http://127.0.0.1:8086',
            changeOrigin: true,
            selfHandleResponse: true,
            configure: (proxy) => {
               proxy.on('proxyRes', (proxyRes, _req, res) => {
                  // Handle upstream errors
                  proxyRes.on('error', () => {
                     if (!res.headersSent) {
                        res.writeHead(502);
                        res.end(JSON.stringify({ message: 'Upstream error' }));
                     }
                  });
                  // Handle client disconnect
                  res.on('close', () => {
                     proxyRes.destroy();
                  });
                  res.writeHead(proxyRes.statusCode || 200, {
                     'Content-Type': 'text/event-stream',
                     'Cache-Control': 'no-cache',
                     'Connection': 'keep-alive',
                     ...proxyRes.headers,
                  });
                  proxyRes.pipe(res);
               });
               // Handle proxy-level errors (e.g. connection refused)
               proxy.on('error', (_err, _req, res: any) => {
                  if (res?.writeHead) {
                     res.writeHead(502, { 'Content-Type': 'application/json' });
                     res.end(JSON.stringify({ message: 'Proxy error' }));
                  }
               });
            },
         },
         '/api': {
            target: 'http://127.0.0.1:8086',
            changeOrigin: true,
         },
      },
   },
   test: {
      globals: true,
      environment: 'jsdom',
      setupFiles: './src/test/setup.ts',
      css: true,
   },
});
