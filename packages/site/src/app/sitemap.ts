import type { MetadataRoute } from "next";
import { installationDocs, legalDocuments } from "@/content/metadata";

const BASE_URL = "https://rokbattles.com";

export default function sitemap(): MetadataRoute.Sitemap {
  const lastModified = new Date().toISOString().split("T")[0];
  const routes = [
    "",
    "/olympian-arena",
    "/combat-lab",
    "/combat-lab/rankings",
    "/loot-explorer/barbarians",
    "/loot-explorer/barbarian-forts",
    "/loot-explorer/baulurs",
    "/loot-explorer/karuak-ceremony",
    "/loot-explorer/kahars-treasure",
    "/legal",
  ];

  installationDocs.map(({ slug }) => routes.push(`/docs/installation/${slug}`));
  legalDocuments.map(({ id }) => routes.push(`/legal/${id}`));

  return routes.map((route) => ({
    url: `${BASE_URL}${route}`,
    lastModified,
  }));
}
