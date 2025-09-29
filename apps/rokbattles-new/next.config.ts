import type { NextConfig } from "next";
import createNextIntlPlugin from "next-intl/plugin";

const plugins = [createNextIntlPlugin()];
const isProvEnv = process.env.NODE_ENV === "production";

const config: NextConfig = {
  reactStrictMode: !isProvEnv,
  typedRoutes: true,
};

module.exports = () => plugins.reduce((acc, next) => next(acc), config);
