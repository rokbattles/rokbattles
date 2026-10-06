declare module "*.md" {
  import type { MDXContent } from "mdx/types";

  const Content: MDXContent;
  export default Content;
}

declare module "*.mdx" {
  import type { MDXContent } from "mdx/types";

  const Content: MDXContent;
  export default Content;
}
