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
const guiPort = parseInt(
  process.env.GUI_PORT ??
  String((conf.gui as Record<string, unknown>)?.port ?? 3030)
);

export default defineConfig({
  plugins: [solid(), tailwindcss()],
  server: {
    port: guiPort,
  },
});
