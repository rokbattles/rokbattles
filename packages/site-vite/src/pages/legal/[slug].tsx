import clsx from "clsx";
import { ArrowLeft } from "lucide-react";
import { Suspense } from "react";
import { MarkdownDocument } from "../../components/markdown-document";
import Metadata from "../../components/metadata";
import { gutter } from "../../components/ui/marketing/layout";
import { Link } from "../../components/ui/marketing/link";
import { Text } from "../../components/ui/marketing/text";
import type { legalDocuments } from "../../lib/legal-documents";

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
          <MarkdownDocument Content={document.Content} title={document.title} />
        </Suspense>
      </div>
    </div>
  );
}
