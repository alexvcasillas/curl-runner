import { fileURLToPath } from 'node:url';
import react from '@vitejs/plugin-react';
import { fumadocsMdx } from 'fumadocs-mdx/vite';
import { defineConfig } from 'vitest/config';

export default defineConfig({
  // Compile content/docs/*.mdx for the `.source` index that `fumadocs-mdx` generates on
  // postinstall/build; `index: false` keeps that Next.js-flavoured index as is.
  plugins: [react(), fumadocsMdx({ index: false })],
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./', import.meta.url)),
    },
  },
  test: {
    environment: 'jsdom',
    globals: true,
    setupFiles: ['./tests/setup.ts'],
    include: ['tests/**/*.test.{ts,tsx}'],
  },
});
