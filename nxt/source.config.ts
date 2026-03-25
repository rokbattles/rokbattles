import { pageSchema } from "fumadocs-core/source/schema";
import { defineConfig, defineDocs } from "fumadocs-mdx/config";
import { z } from "zod";

export const docs = defineDocs({
  dir: "src/content/docs",
  docs: {
    schema: pageSchema.extend({
      index: z.boolean().default(false),
    }),
  },
});

export default defineConfig();
