// Control characters, which browsers strip from URLs (`/\t/evil` becomes `//evil`).
function hasControl(s: string): boolean {
  for (let i = 0; i < s.length; i++) {
    const c = s.charCodeAt(i);
    if (c < 0x20 || c === 0x7f) {
      return true;
    }
  }
  return false;
}

function isLocalPath(path: string): boolean {
  return (
    path.startsWith('/') && !path.startsWith('//') && !path.includes('\\') && !hasControl(path)
  );
}

/**
 * `return_to` as a local path, or `/`. The only helper for `return_to`: it must
 * start with `/`, not `//`, and have no `\`, both as given and once decoded.
 */
export function safeReturnTo(value: string | null | undefined): string {
  if (!value) {
    return '/';
  }
  let decoded: string;
  try {
    decoded = decodeURIComponent(value);
  } catch {
    return '/';
  }
  return isLocalPath(value) && isLocalPath(decoded) ? value : '/';
}
