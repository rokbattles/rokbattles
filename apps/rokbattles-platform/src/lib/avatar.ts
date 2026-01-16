export function getInitials(name: string): string | undefined {
  const tokens = name.split(/\s+/).filter(Boolean);
  if (tokens.length === 0) return undefined;
  if (tokens.length === 1) return tokens[0]?.slice(0, 2).toUpperCase();
  return (
    tokens[0]?.slice(0, 1) + tokens[tokens.length - 1]?.slice(0, 1)
  ).toUpperCase();
}

export function normalizeFrameUrl(
  frameUrl?: string | null
): string | undefined {
  if (typeof frameUrl !== "string") return undefined;
  const trimmed = frameUrl.trim();
  if (trimmed.length === 0 || trimmed.toLowerCase() === "null")
    return undefined;
  return trimmed.replace("http://", "https://");
}
