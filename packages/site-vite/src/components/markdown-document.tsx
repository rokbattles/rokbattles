import type { MDXComponents, MDXProps } from "mdx/types";
import type { ComponentType } from "react";
import { Heading } from "./ui/marketing/heading";

const components: MDXComponents = {
  h1: (props) => <Heading {...props} level={1} className="mb-8" />,
};

export function MarkdownDocument({
  Content,
  title,
  components: overrides,
}: {
  Content: ComponentType<MDXProps>;
  title: string;
  components?: MDXComponents;
}) {
  return (
    <article
      aria-label={title}
      className="text-base/8 wrap-break-word text-zinc-300 [&_a]:text-orange-400 [&_a]:underline [&_a]:underline-offset-4 [&_a:focus-visible]:outline-2 [&_a:focus-visible]:outline-offset-4 [&_a:focus-visible]:outline-orange-400 [&_a:hover]:text-orange-300 [&_blockquote]:my-6 [&_blockquote]:border-l-2 [&_blockquote]:border-white/20 [&_blockquote]:pl-5 [&_h2]:mt-10 [&_h2]:mb-4 [&_h2]:text-2xl [&_h2]:font-medium [&_h2]:text-white [&_h3]:mt-8 [&_h3]:mb-3 [&_h3]:text-xl [&_h3]:font-medium [&_h3]:text-white [&_hr]:my-8 [&_hr]:border-white/10 [&_li]:my-2 [&_ol]:my-5 [&_ol]:list-decimal [&_ol]:pl-6 [&_p]:my-5 [&_strong]:font-semibold [&_strong]:text-white [&_ul]:my-5 [&_ul]:list-disc [&_ul]:pl-6 [&_code]:rounded [&_code]:bg-white/5 [&_code]:px-1.5 [&_code]:py-0.5 [&_code]:text-sm [&_pre]:my-6 [&_pre]:overflow-x-auto [&_pre]:rounded-lg [&_pre]:border [&_pre]:border-white/10 [&_pre]:bg-white/5 [&_pre]:p-4 [&_pre_code]:bg-transparent [&_pre_code]:p-0"
    >
      <Content components={{ ...components, ...overrides }} />
    </article>
  );
}
