"use server";

import * as fsSync from "node:fs";
import fs from "node:fs/promises";
import path from "node:path";
import type { Locale } from "next-intl";
import { cache } from "react";
import { parse as parseYAML } from "yaml";
import { routing } from "@/i18n/routing";

const DEFAULT_LOCALE: Locale = routing.defaultLocale;

type LocalizedName = {
  en?: string;
  es?: string;
  kr?: string;
  [key: string]: string | undefined;
};

type DatasetEntry = {
  name?: LocalizedName;
  [key: string]: unknown;
};

type DatasetMap = Record<string, DatasetEntry>;

function getDatasetsBasePath(): string {
  const envPath = process.env.ROKB_DATASETS_PATH;
  if (envPath && envPath.trim().length > 0) {
    return path.resolve(envPath);
  }

  // local development
  const cwd = process.cwd();
  const candidates = [path.resolve(cwd, "datasets"), path.resolve(cwd, "..", "..", "datasets")];

  return (
    candidates.find((p) => {
      try {
        return fsSync.existsSync(p);
      } catch {
        return false;
      }
    }) ?? candidates[0]
  );
}

// biome-ignore lint/suspicious/noExplicitAny: ignore
async function readYamlFile(filePath: string): Promise<any> {
  const content = await fs.readFile(filePath, "utf-8");
  return parseYAML(content);
}

const loadDataset = cache(async (datasetName: string): Promise<DatasetMap> => {
  const base = getDatasetsBasePath();
  const filename = path.join(base, `${datasetName}.yaml`);
  // biome-ignore lint/suspicious/noExplicitAny: ignore
  let raw: any;
  try {
    raw = await readYamlFile(filename);
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    throw new Error(`Failed to read dataset '${datasetName}' at ${filename}: ${msg}`);
  }
  // biome-ignore lint/suspicious/noExplicitAny: ignore
  const inner: any = raw?.[datasetName] ?? raw;
  if (!inner || typeof inner !== "object") {
    throw new Error(`Dataset '${datasetName}' has unexpected structure in ${filename}`);
  }
  if (datasetName === "equipment") {
    // biome-ignore lint/suspicious/noExplicitAny: ignore
    const item = (inner as any)?.item;
    if (item && typeof item === "object") {
      return item as DatasetMap;
    }
  }
  return inner as DatasetMap;
});

function pickLocalizedName(entry: DatasetEntry | undefined, locale: Locale): string | undefined {
  if (!entry) return undefined;
  const names = entry.name ?? ({} as LocalizedName);
  if (names[locale]) return names[locale];
  const order: string[] = ["en", "es", "kr"];
  for (const l of order) {
    if (names[l]) return names[l];
  }
  for (const v of Object.values(names)) {
    if (typeof v === "string" && v.length > 0) return v;
  }
  return undefined;
}

export async function resolveName(
  datasetName: string,
  id: string | number,
  locale: Locale = DEFAULT_LOCALE
): Promise<string | undefined> {
  const ds = await loadDataset(datasetName);
  const key = String(id);
  const entry = ds[key];
  return pickLocalizedName(entry, locale);
}

export async function resolveNames(
  datasetName: string,
  ids: Array<string | number>,
  locale: Locale = DEFAULT_LOCALE
): Promise<Record<string, string | undefined>> {
  const ds = await loadDataset(datasetName);
  const out: Record<string, string | undefined> = {};
  for (const id of ids) {
    const key = String(id);
    out[key] = pickLocalizedName(ds[key], locale);
  }
  return out;
}

export async function getEntry(
  datasetName: string,
  id: string | number
): Promise<DatasetEntry | undefined> {
  const ds = await loadDataset(datasetName);
  return ds[String(id)];
}

export async function resolveProperty<T = unknown>(
  datasetName: string,
  id: string | number,
  prop: string
): Promise<T | undefined> {
  const entry = await getEntry(datasetName, id);
  if (!entry) return undefined;
  // biome-ignore lint/suspicious/noExplicitAny: ignore
  return (entry as any)[prop] as T | undefined;
}
