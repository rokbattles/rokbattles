import type { NextConfig } from "next";

const plugins = [];
const isProvEnv = process.env.NODE_ENV === "production";

const config: NextConfig = {
  reactStrictMode: !isProvEnv,
  typedRoutes: true,
};

module.exports = () => plugins.reduce((acc, next) => next(acc), config);
