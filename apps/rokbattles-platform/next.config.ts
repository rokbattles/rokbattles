import type { NextConfig } from "next";
import createNextIntlPlugin from "next-intl/plugin";

const withNextIntl = createNextIntlPlugin({
  experimental: {
    srcPath: "./src",
    extract: {
      sourceLocale: "en",
    },
    messages: {
      path: "./src/i18n/messages",
      format: "po",
      locales: "infer",
      precompile: true,
    },
  },
});

const config: NextConfig = {};

export default withNextIntl(config);
