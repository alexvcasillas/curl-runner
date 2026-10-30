import Image from 'next/image';
import Link from 'next/link';
import type { ReactNode } from 'react';
import {
  type Screenshot as ScreenshotData,
  STUDIO_DOCS_HREF,
  type StudioScreenshotName,
  studioScreenshot,
} from '@/lib/studio';

/** Screenshots are mostly text: a higher quality than the default 75 keeps it crisp. */
export const SCREENSHOT_QUALITY = 90;

interface ScreenshotFrameProps {
  shot: ScreenshotData;
  /** `sizes` for next/image: the frame's rendered width at each breakpoint. */
  sizes: string;
  caption?: ReactNode;
  className?: string;
  captionClassName?: string;
}

/** An app screenshot in the site's pane frame (hairline border, 14px radius), with a caption. */
export function ScreenshotFrame({
  shot,
  sizes,
  caption,
  className = '',
  captionClassName = 'mt-3.5 text-[13px] leading-[1.6] text-fg-tertiary',
}: ScreenshotFrameProps) {
  return (
    <figure className={`not-prose m-0 min-w-0 ${className}`}>
      <div className="overflow-hidden rounded-[14px] border border-line bg-ground">
        <Image
          src={shot.src}
          width={shot.width}
          height={shot.height}
          alt={shot.alt}
          sizes={sizes}
          quality={SCREENSHOT_QUALITY}
          className="block h-auto w-full"
        />
      </div>
      {caption ? <figcaption className={captionClassName}>{caption}</figcaption> : null}
    </figure>
  );
}

/**
 * A studio screenshot in a docs page (`<StudioScreenshot name="…">caption</StudioScreenshot>`).
 * The caption always says it is the studio, and that the studio is a preview.
 */
export function StudioScreenshot({
  name,
  children,
  onStudioPage = false,
}: {
  name: StudioScreenshotName;
  children?: ReactNode;
  /** On the studio's own docs page the caption needn't say where the screenshot is from. */
  onStudioPage?: boolean;
}) {
  return (
    <ScreenshotFrame
      shot={studioScreenshot(name)}
      sizes="(min-width: 1280px) 840px, (min-width: 768px) calc(100vw - 320px), 100vw"
      className="my-8"
      caption={
        onStudioPage ? (
          children
        ) : (
          <>
            {children}
            {children ? ' ' : null}
            Shown in{' '}
            <Link
              href={STUDIO_DOCS_HREF}
              className="text-fg-secondary underline underline-offset-4 transition-colors hover:text-fg"
            >
              curl-runner studio
            </Link>
            , a macOS preview.
          </>
        )
      }
    />
  );
}
