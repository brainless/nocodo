import { defineConfig } from 'vite';
import solid from 'vite-plugin-solid';
import tailwindcss from '@tailwindcss/vite';
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const _dir = dirname(fileURLToPath(import.meta.url));

function readProjectConf(key: string): string | undefined {
  for (const rel of ['../project.conf', '../../project.conf']) {
    try {
      const content = readFileSync(join(_dir, rel), 'utf-8');
      for (const line of content.split('\n')) {
        const t = line.trim();
        if (!t || t.startsWith('#')) continue;
        const i = t.indexOf('=');
        if (i < 0) continue;
        if (t.slice(0, i).trim() === key) {
          const v = t.slice(i + 1).trim().replace(/^['"]|['"]$/g, '');
          if (v) return v;
        }
      }
    } catch { /* project.conf not found at this path */ }
  }
}

const adminGuiPort = parseInt(process.env.ADMIN_GUI_PORT ?? readProjectConf('ADMIN_GUI_PORT') ?? '3031');
const backendPort = parseInt(process.env.BACKEND_PORT ?? readProjectConf('BACKEND_PORT') ?? '8080');

export default defineConfig({
  plugins: [
    solid(),
    tailwindcss(),
    {
      name: 'admin-base-rewrite',
      configureServer(server) {
        server.middlewares.use((req, _res, next) => {
          if (req.url === '/admin') req.url = '/admin/';
          next();
        });
      },
    },
  ],
  base: '/admin/',
  server: {
    port: adminGuiPort,
    proxy: {
      '/api': `http://127.0.0.1:${backendPort}`,
    },
  },
});
