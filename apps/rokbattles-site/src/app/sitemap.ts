import type { MetadataRoute } from "next";

export default function sitemap(): MetadataRoute.Sitemap {
  return ["", "/live"].map((route) => ({
    url: `https://rokbattles.com${route}`,
    lastModified: new Date().toISOString().split("T")[0],
  }));
}
