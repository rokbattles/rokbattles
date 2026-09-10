import clsx from "clsx";
import { ArrowLeft } from "lucide-react";
import type { MDXComponents } from "mdx/types";
import { Suspense } from "react";
import Metadata from "../../components/metadata";
import { Heading } from "../../components/ui/marketing/heading";
import { gutter } from "../../components/ui/marketing/layout";
import { Link } from "../../components/ui/marketing/link";
import { Text } from "../../components/ui/marketing/text";
import type { legalDocuments } from "../../lib/legal-documents";

const components: MDXComponents = {
  h1: (props) => <Heading {...props} level={1} className="mb-8" />,
};

export default function LegalDocumentRoute({
  document,
}: {
  document: (typeof legalDocuments)[number];
}) {
  return (
    <div className={clsx(gutter, "border-b border-white/10 py-10 sm:py-12")}>
      <div className="mx-auto max-w-3xl">
        <Link href="/legal" className="mb-8">
          <ArrowLeft aria-hidden="true" /> All legal documents
        </Link>
        <Metadata title={document.title} description={document.description} />
        <Suspense key={document.id} fallback={<Text role="status">Loading document...</Text>}>
          <article
            aria-label={document.title}
            className="text-base/8 wrap-break-word text-zinc-300 [&_a]:text-orange-400 [&_a]:underline [&_a]:underline-offset-4 [&_a:focus-visible]:outline-2 [&_a:focus-visible]:outline-offset-4 [&_a:focus-visible]:outline-orange-400 [&_a:hover]:text-orange-300 [&_blockquote]:my-6 [&_blockquote]:border-l-2 [&_blockquote]:border-white/20 [&_blockquote]:pl-5 [&_h2]:mt-10 [&_h2]:mb-4 [&_h2]:text-2xl [&_h2]:font-medium [&_h2]:text-white [&_h3]:mt-8 [&_h3]:mb-3 [&_h3]:text-xl [&_h3]:font-medium [&_h3]:text-white [&_hr]:my-8 [&_hr]:border-white/10 [&_li]:my-2 [&_ol]:my-5 [&_ol]:list-decimal [&_ol]:pl-6 [&_p]:my-5 [&_strong]:font-semibold [&_strong]:text-white [&_ul]:my-5 [&_ul]:list-disc [&_ul]:pl-6"
          >
            <document.Content components={components} />
          </article>
        </Suspense>
      </div>
    </div>
  );
}
