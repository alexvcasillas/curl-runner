import { readFile } from 'node:fs/promises';
import { join } from 'node:path';

// Geist TTFs for OG images (Satori reads TTF, OTF and WOFF, not WOFF2). They are copies of the
// files in the `geist` package. That package ships no non-italic ExtraBold, so 800 uses Bold.
export async function getOgFonts() {
  const fontsDir = join(process.cwd(), 'public', 'fonts');

  const [regular, bold] = await Promise.all([
    readFile(join(fontsDir, 'Geist-Regular.ttf')),
    readFile(join(fontsDir, 'Geist-Bold.ttf')),
  ]);

  return [
    { name: 'Geist', data: regular, weight: 400 as const, style: 'normal' as const },
    { name: 'Geist', data: bold, weight: 700 as const, style: 'normal' as const },
    { name: 'Geist', data: bold, weight: 800 as const, style: 'normal' as const },
  ];
}
