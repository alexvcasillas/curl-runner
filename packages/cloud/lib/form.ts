/** A trimmed string field from a form, or "" when it is missing or a file. */
export function text(formData: FormData, name: string): string {
  const v = formData.get(name);
  return typeof v === 'string' ? v.trim() : '';
}
