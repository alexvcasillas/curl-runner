import { readFile } from 'node:fs/promises';
import { join } from 'node:path';

type FontWeight = 100 | 200 | 300 | 400 | 500 | 600 | 700 | 800 | 900;
type FontStyle = 'normal' | 'italic';

interface FontOptions {
  name: string;
  data: Buffer;
  weight: FontWeight;
  style: FontStyle;
}

let fontsCache: FontOptions[] | null = null;

export async function getOgFonts(): Promise<FontOptions[]> {
  if (fontsCache) {
    return fontsCache;
  }

  const fontsDir = join(process.cwd(), 'lib', 'fonts');

  const [interRegular, interBold] = await Promise.all([
    readFile(join(fontsDir, 'Inter-Regular.woff')),
    readFile(join(fontsDir, 'Inter-Bold.woff')),
  ]);

  fontsCache = [
    { name: 'Inter', data: interRegular, weight: 400 as FontWeight, style: 'normal' as FontStyle },
    { name: 'Inter', data: interBold, weight: 700 as FontWeight, style: 'normal' as FontStyle },
  ];

  return fontsCache;
}

export async function getOgIcon() {
  const iconData = await readFile(join(process.cwd(), 'public', 'icon-light-192.png'));
  return `data:image/png;base64,${iconData.toString('base64')}`;
}
