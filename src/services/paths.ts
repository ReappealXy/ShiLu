/** Keep Windows canonical paths for disk operations; adapt only the display/URL boundary. */
export function displayPath(value: string): string {
  if (value.startsWith("\\\\?\\UNC\\")) return `\\\\${value.slice(8)}`;
  return value.startsWith("\\\\?\\") ? value.slice(4) : value;
}
