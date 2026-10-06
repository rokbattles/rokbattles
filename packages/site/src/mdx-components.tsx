import type { MDXComponents } from "mdx/types";
import { SiteLink } from "@/components/(marketing)/site-link";
import { version } from "../package.json";

const components: MDXComponents = {
  a: ({ href, ...props }) => <SiteLink href={href ?? "#"} {...props} />,
  code: ({ children, ...props }) => (
    <code {...props}>
      {typeof children === "string" ? children.replaceAll("__VERSION__", version) : children}
    </code>
  ),
};

export function useMDXComponents(): MDXComponents {
  return components;
}
