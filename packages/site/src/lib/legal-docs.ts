import { lstat, readFile, realpath } from "node:fs/promises";
import { isAbsolute, relative, resolve } from "node:path";

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
const legalBasePath = resolve(process.cwd(), "legal");

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

  const filePath = resolve(legalBasePath, doc.filename);

  try {
    const safeFilePath = await getSafeLegalDocumentPath(filePath);
    const content = await readFile(safeFilePath, "utf-8");
    return { ...doc, content };
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    throw new Error(`Failed to read legal document '${slug}' from ${filePath}: ${message}`);
  }
}

async function getSafeLegalDocumentPath(filePath: string): Promise<string> {
  const baseStats = await lstat(legalBasePath);
  if (!baseStats.isDirectory() || baseStats.isSymbolicLink()) {
    throw new Error(`Legal documents path is not a real directory: ${legalBasePath}`);
  }

  const fileStats = await lstat(filePath);
  if (!fileStats.isFile() || fileStats.isSymbolicLink()) {
    throw new Error(`Legal document is not a regular file: ${filePath}`);
  }

  const baseRealPath = await realpath(legalBasePath);
  const fileRealPath = await realpath(filePath);
  const relativePath = relative(baseRealPath, fileRealPath);

  if (relativePath.startsWith("..") || isAbsolute(relativePath) || relativePath === "") {
    throw new Error(`Legal document resolves outside legal directory: ${filePath}`);
  }

  return fileRealPath;
}
