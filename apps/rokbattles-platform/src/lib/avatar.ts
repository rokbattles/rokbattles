const HTTP_URL_PREFIX_REGEX = /^http:\/\//i;

export function rewriteHttpToHttps(url?: string | null): string | undefined {
  if (typeof url !== "string") {
    return undefined;
  }

  const trimmed = url.trim();
  if (trimmed.length === 0 || trimmed.toLowerCase() === "null") {
    return undefined;
  }

  return trimmed.replace(HTTP_URL_PREFIX_REGEX, "https://");
}
