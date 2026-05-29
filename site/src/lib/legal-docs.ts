import { readFile } from "node:fs/promises";
import { join } from "node:path";

export type LegalDocument = {
  slug: string;
  title: string;
  filename: string;
};

const LEGAL_DOCUMENTS: LegalDocument[] = [
  {
    slug: "terms-of-service",
    title: "Terms of Service",
    filename: "terms-of-service.md",
  },
  {
    slug: "privacy-policy",
    title: "Privacy Policy",
    filename: "privacy-policy.md",
  },
  {
    slug: "cookie-policy",
    title: "Cookie Policy",
    filename: "cookie-policy.md",
  },
];

const documentsBySlug = new Map(LEGAL_DOCUMENTS.map((doc) => [doc.slug, doc]));
const legalBasePath = join(process.cwd(), "legal");

export function getLegalDocuments(): readonly LegalDocument[] {
  return LEGAL_DOCUMENTS;
}

export function getLegalDocument(slug: string): LegalDocument | undefined {
  return documentsBySlug.get(slug);
}

export async function loadLegalDocument(
  slug: string
): Promise<(LegalDocument & { content: string }) | undefined> {
  const doc = getLegalDocument(slug);
  if (!doc) return undefined;

  const filePath = join(legalBasePath, doc.filename);

  try {
    const content = await readFile(filePath, "utf-8");
    return { ...doc, content };
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    throw new Error(`Failed to read legal document '${slug}' from ${filePath}: ${message}`);
  }
}
