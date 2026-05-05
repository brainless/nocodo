import { defineConfig } from 'vite';
import solid from 'vite-plugin-solid';
import tailwindcss from '@tailwindcss/vite';
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { parse } from 'smol-toml';

const _dir = dirname(fileURLToPath(import.meta.url));

function readProjectToml(): Record<string, unknown> {
  for (const rel of ['../project.toml', '../../project.toml']) {
    try {
      return parse(readFileSync(join(_dir, rel), 'utf-8'));
    } catch { /* not found at this path */ }
  }
  return {};
}

const conf = readProjectToml();
const adminGuiPort = parseInt(
  process.env.ADMIN_GUI_PORT ??
  String((conf.admin_gui as Record<string, unknown>)?.port ?? 3031)
);
const backendPort = parseInt(
  process.env.BACKEND_PORT ??
  String((conf.server as Record<string, unknown>)?.port ?? 8080)
);

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
    host: '127.0.0.1',
    port: adminGuiPort,
    proxy: {
      '/api': `http://127.0.0.1:${backendPort}`,
    },
  },
});
