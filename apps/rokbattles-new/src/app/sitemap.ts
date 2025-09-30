import type { MetadataRoute } from "next";

export default function sitemap(): MetadataRoute.Sitemap {
  const lastModified = new Date().toISOString();
  const routes = [""];

  return routes.map((route) => ({
    url: `https://rokbattles.com${route}`,
    lastModified,
  }));
}
