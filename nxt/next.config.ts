import { createMDX } from "fumadocs-mdx/next";
import type { NextConfig } from "next";

const withMDX = createMDX({});

const plugins = [withMDX];
const isProdEnv = process.env.NODE_ENV === "production";

const config: NextConfig = {
  reactStrictMode: !isProdEnv,
};

module.exports = plugins.reduce((acc, next) => next(acc), config);
