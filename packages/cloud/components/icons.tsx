// The boards' own 16×16 glyphs, so the kit matches them stroke for stroke.
import type { SVGProps } from 'react';

type IconProps = Omit<SVGProps<SVGSVGElement>, 'children'> & { size?: number };

function Glyph({
  size = 14,
  strokeWidth = 1.5,
  children,
  ...rest
}: IconProps & { children: React.ReactNode }) {
  return (
    <svg
      aria-hidden="true"
      width={size}
      height={size}
      viewBox="0 0 16 16"
      fill="none"
      stroke="currentColor"
      strokeWidth={strokeWidth}
      {...rest}
    >
      {children}
    </svg>
  );
}

export function InfoIcon(p: IconProps) {
  return (
    <Glyph {...p}>
      <circle cx="8" cy="8" r="6" />
      <path d="M8 7.5V11M8 5h.01" />
    </Glyph>
  );
}

export function CheckIcon(p: IconProps) {
  return (
    <Glyph strokeWidth={1.6} strokeLinecap="round" strokeLinejoin="round" {...p}>
      <path d="M3 8.5l3 3 7-7" />
    </Glyph>
  );
}

export function WarningIcon(p: IconProps) {
  return (
    <Glyph strokeLinejoin="round" {...p}>
      <path d="M8 2l6.5 11.5h-13z" />
      <path d="M8 6.5v3M8 11.5h.01" />
    </Glyph>
  );
}

export function ErrorIcon(p: IconProps) {
  return (
    <Glyph strokeLinecap="round" {...p}>
      <circle cx="8" cy="8" r="6" />
      <path d="M8 5v3.5M8 11h.01" />
    </Glyph>
  );
}

export function ChevronsUpDownIcon(p: IconProps) {
  return (
    <Glyph strokeLinecap="round" strokeLinejoin="round" {...p}>
      <path d="M5.5 6L8 3.5 10.5 6M5.5 10L8 12.5 10.5 10" />
    </Glyph>
  );
}

export function ChevronDownIcon(p: IconProps) {
  return (
    <Glyph {...p}>
      <path d="M5 6l3 3 3-3" />
    </Glyph>
  );
}

export function ChevronRightIcon(p: IconProps) {
  return (
    <Glyph strokeLinecap="round" {...p}>
      <path d="M6 3.5L10.5 8 6 12.5" />
    </Glyph>
  );
}

export function MenuIcon(p: IconProps) {
  return (
    <Glyph strokeLinecap="round" {...p}>
      <path d="M2.5 4.5h11M2.5 8h11M2.5 11.5h11" />
    </Glyph>
  );
}

export function SearchIcon(p: IconProps) {
  return (
    <Glyph {...p}>
      <circle cx="7" cy="7" r="4.5" />
      <path d="M10.5 10.5L14 14" />
    </Glyph>
  );
}

export function FolderIcon(p: IconProps) {
  return (
    <Glyph strokeLinecap="round" {...p}>
      <path d="M2 4.5h4l1.5 1.5H14v6.5H2z" />
    </Glyph>
  );
}

export function PlusIcon(p: IconProps) {
  return (
    <Glyph strokeWidth={1.6} strokeLinecap="round" {...p}>
      <path d="M8 3v10M3 8h10" />
    </Glyph>
  );
}

export function ShieldIcon(p: IconProps) {
  return (
    <Glyph strokeLinejoin="round" {...p}>
      <path d="M8 1.5l5 2v4c0 3.2-2.2 5.6-5 7-2.8-1.4-5-3.8-5-7v-4z" />
    </Glyph>
  );
}

export function MenuCheckIcon(p: IconProps) {
  return (
    <Glyph strokeLinecap="round" strokeLinejoin="round" size={13} {...p}>
      <path d="M3.5 8.5l3 3 6-7" />
    </Glyph>
  );
}

export function GlobeIcon(p: IconProps) {
  return (
    <Glyph strokeLinecap="round" strokeLinejoin="round" {...p}>
      <path d="M8 2a6 6 0 100 12A6 6 0 008 2zM2 8h12M8 2c1.7 1.8 2.5 3.8 2.5 6s-.8 4.2-2.5 6c-1.7-1.8-2.5-3.8-2.5-6S6.3 3.8 8 2z" />
    </Glyph>
  );
}

export function LockIcon(p: IconProps) {
  return (
    <Glyph strokeLinecap="round" strokeLinejoin="round" {...p}>
      <path d="M4.5 7.5h7v6h-7zM5.5 7.5V5.5a2.5 2.5 0 015 0v2" />
    </Glyph>
  );
}

export function EyeOffIcon(p: IconProps) {
  return (
    <Glyph strokeLinecap="round" strokeLinejoin="round" {...p}>
      <path d="M2 2l12 12M6.5 6.6A2 2 0 009.4 9.5M4 4.5C2.7 5.5 2 7 2 8c1 2.2 3.3 4 6 4 1 0 2-.3 2.9-.8M7 4.1c.3 0 .7-.1 1-.1 2.7 0 5 1.8 6 4-.3.8-.8 1.5-1.4 2.1" />
    </Glyph>
  );
}

export function GitIcon(p: IconProps) {
  return (
    <Glyph strokeLinecap="round" {...p}>
      <circle cx="4.5" cy="4" r="1.5" />
      <circle cx="4.5" cy="12" r="1.5" />
      <circle cx="11.5" cy="6" r="1.5" />
      <path d="M4.5 5.5v5M11.5 7.5c0 2-2 2.5-7 3" />
    </Glyph>
  );
}

export function PeopleIcon(p: IconProps) {
  return (
    <Glyph strokeLinecap="round" {...p}>
      <circle cx="6" cy="5.5" r="2.5" />
      <path d="M1.5 13.5c.6-2.3 2.3-3.5 4.5-3.5s3.9 1.2 4.5 3.5M11 3.5a2.5 2.5 0 010 4.5M12.5 10.5c1 .6 1.7 1.6 2 3" />
    </Glyph>
  );
}

export function ClockIcon(p: IconProps) {
  return (
    <Glyph strokeLinecap="round" {...p}>
      <path d="M8 4.5V8l2.5 1.5" />
      <circle cx="8" cy="8" r="5.5" />
    </Glyph>
  );
}
