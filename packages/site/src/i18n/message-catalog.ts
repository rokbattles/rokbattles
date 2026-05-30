export type MessageTree = Record<string, unknown>;

const isMessageTree = (value: unknown): value is MessageTree =>
  Boolean(value) && typeof value === "object" && !Array.isArray(value);

const isMissingCatalogError = (error: unknown): boolean =>
  error instanceof Error &&
  (error.message.includes("Cannot find module") ||
    (typeof (error as { code?: unknown }).code === "string" &&
      (error as { code?: string }).code === "MODULE_NOT_FOUND"));

export const loadCatalogMessages = async (locale: string): Promise<MessageTree | undefined> => {
  try {
    return (await import(`./messages/${locale}.po`)).default as MessageTree;
  } catch (error) {
    if (isMissingCatalogError(error)) {
      return undefined;
    }

    throw error;
  }
};

export const toIntlLocale = (locale: string) => locale.replace("_", "-");

export const mergeMessagesWithFallback = (
  localizedMessages: MessageTree,
  fallbackMessages: MessageTree
): MessageTree => {
  const merged: MessageTree = {};
  const keys = new Set([...Object.keys(fallbackMessages), ...Object.keys(localizedMessages)]);

  for (const key of keys) {
    const localizedValue = localizedMessages[key];
    const fallbackValue = fallbackMessages[key];

    if (localizedValue === "" || localizedValue == null) {
      merged[key] = fallbackValue;
      continue;
    }

    if (isMessageTree(localizedValue) && isMessageTree(fallbackValue)) {
      merged[key] = mergeMessagesWithFallback(localizedValue, fallbackValue);
      continue;
    }

    merged[key] = localizedValue;
  }

  return merged;
};
