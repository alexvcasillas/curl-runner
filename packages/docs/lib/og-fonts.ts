import { readFile } from 'node:fs/promises';
import { join } from 'node:path';

let fontsCache: { name: string; data: Buffer; weight: number; style: string }[] | null = null;

export async function getOgFonts() {
  if (fontsCache) {
    return fontsCache;
  }

  const fontsDir = join(process.cwd(), 'lib', 'fonts');

  const [interRegular, interBold] = await Promise.all([
    readFile(join(fontsDir, 'Inter-Regular.woff')),
    readFile(join(fontsDir, 'Inter-Bold.woff')),
  ]);

  fontsCache = [
    { name: 'Inter', data: interRegular, weight: 400, style: 'normal' },
    { name: 'Inter', data: interBold, weight: 700, style: 'normal' },
  ];

  return fontsCache;
}

export async function getOgIcon() {
  const iconData = await readFile(join(process.cwd(), 'public', 'icon-light-192.png'));
  return `data:image/png;base64,${iconData.toString('base64')}`;
}
