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

const guiPort = parseInt(process.env.GUI_PORT ?? readProjectConf('GUI_PORT') ?? '3030');

export default defineConfig({
  plugins: [solid(), tailwindcss()],
  server: {
    port: guiPort,
  },
});
