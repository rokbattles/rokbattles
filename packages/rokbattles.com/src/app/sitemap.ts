import type { MetadataRoute } from "next";
import { installationDocs, legalDocuments } from "@/content/metadata";

export const dynamic = "force-static";

export default function sitemap(): MetadataRoute.Sitemap {
  return [
    "/",
    "/docs/",
    ...installationDocs.map(({ slug }) => `/docs/installation/${slug}/`),
    "/legal/",
    ...legalDocuments.map(({ id }) => `/legal/${id}/`),
  ].map((path) => ({ url: `https://rokbattles.com${path}` }));
}
