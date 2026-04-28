import type { MetadataRoute } from "next";
import { getLegalDocuments } from "@/lib/legal-docs";
import { source } from "@/lib/source";

const BASE_URL = "https://rokbattles.com";

export default function sitemap(): MetadataRoute.Sitemap {
  const lastModified = new Date().toISOString().split("T")[0];
  const routes = ["", "/olympian-arena", "/legal"];

  getLegalDocuments().map((doc) => routes.push(`/legal/${doc.slug}`));
  source.getPages().map((page) => routes.push(page.url));

  return routes.map((route) => ({
    url: `${BASE_URL}${route}`,
    lastModified,
  }));
}
