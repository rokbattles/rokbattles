import type { MetadataRoute } from "next";

const BASE_URL = "https://platform.rokbattles.com";

export default function sitemap(): MetadataRoute.Sitemap {
  const lastModified = new Date().toISOString().split("T")[0];
  const routes = ["", "/olympian-arena"];

  return routes.map((route) => ({
    url: `${BASE_URL}${route}`,
    lastModified,
  }));
}
