import { readFile } from 'node:fs/promises';
import { join } from 'node:path';

// Load bundled Inter WOFF fonts for OG images
// Satori supports WOFF (not WOFF2), so we use @fontsource/inter WOFF files
export async function getOgFonts() {
  const fontsDir = join(process.cwd(), 'public', 'fonts');

  const [interRegular, interBold, interExtraBold] = await Promise.all([
    readFile(join(fontsDir, 'inter-latin-400-normal.woff')),
    readFile(join(fontsDir, 'inter-latin-700-normal.woff')),
    readFile(join(fontsDir, 'inter-latin-800-normal.woff')),
  ]);

  return [
    { name: 'Inter', data: interRegular, weight: 400 as const, style: 'normal' as const },
    { name: 'Inter', data: interBold, weight: 700 as const, style: 'normal' as const },
    { name: 'Inter', data: interExtraBold, weight: 800 as const, style: 'normal' as const },
  ];
}

export async function getOgIcon() {
  const iconData = await readFile(join(process.cwd(), 'public', 'icon-light-192.png'));
  return `data:image/png;base64,${iconData.toString('base64')}`;
}
