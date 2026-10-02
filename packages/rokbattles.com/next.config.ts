import createMDX from "@next/mdx";
import type { NextConfig } from "next";

const config: NextConfig = {
  agentRules: false,
  output: "export",
  trailingSlash: true,
  reactCompiler: true,
  images: { unoptimized: true },
};

export default createMDX({ extension: /\.mdx?$/ })(config);
