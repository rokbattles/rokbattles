import { resolve } from "node:path";
import { pathToFileURL } from "node:url";
import type { Plugin } from "vite";

const baseUrl = "https://staging.rokbattles.com";

export function sitemap(): Plugin {
  let root: string;

  return {
    name: "sitemap",
    apply: "build",
    configResolved(config) {
      root = config.root;
    },
    async generateBundle() {
      // Runtime imports keep MDX out of Vite's config bundler. Node reads the
      // document lists without invoking their lazy content loaders.
      const metadataUrl = pathToFileURL(resolve(root, "src/content/metadata.ts")).href;
      const { installationDocs, legalDocuments }: typeof import("../src/content/metadata.ts") =
        await import(metadataUrl);

      const publicPaths = [
        "/",
        "/docs",
        ...installationDocs.map((document) => `/docs/installation/${document.slug}`),
        "/legal",
        ...legalDocuments.map((document) => `/legal/${document.id}`),
      ];
      const urls = publicPaths.map((path) => {
        const url = new URL(path, baseUrl).href.replaceAll("&", "&amp;");
        return `  <url><loc>${url}</loc></url>`;
      });

      this.emitFile({
        type: "asset",
        fileName: "sitemap.xml",
        source: [
          '<?xml version="1.0" encoding="UTF-8"?>',
          '<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">',
          ...urls,
          "</urlset>",
          "",
        ].join("\n"),
      });
      this.emitFile({
        type: "asset",
        fileName: "robots.txt",
        source: `User-agent: *\nAllow: /\n\nSitemap: ${baseUrl}/sitemap.xml\n`,
      });
    },
  };
}
