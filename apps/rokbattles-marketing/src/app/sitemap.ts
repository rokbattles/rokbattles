import type { MetadataRoute } from "next";

import { getLegalDocuments } from "@/lib/legal-docs";

const BASE_URL = "https://rokbattles.com";

export default function sitemap(): MetadataRoute.Sitemap {
  const lastModified = new Date().toISOString().split("T")[0];
  const legalRoutes = getLegalDocuments().map((doc) => `/legal/${doc.slug}`);
  const routes = ["", ...legalRoutes];

  return routes.map((route) => ({
    url: `${BASE_URL}${route}`,
    lastModified,
  }));
}
