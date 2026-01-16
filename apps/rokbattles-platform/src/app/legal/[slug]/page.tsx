import { notFound } from "next/navigation";
import ReactMarkdown from "react-markdown";
import { getLegalDocuments, loadLegalDocument } from "@/lib/legal-docs";

export function generateStaticParams() {
  return getLegalDocuments().map((doc) => ({ slug: doc.slug }));
}

export default async function LegalDocumentPage({ params }: PageProps<"/legal/[slug]">) {
  const { slug } = await params;
  const doc = await loadLegalDocument(slug);

  if (!doc) {
    notFound();
  }

  return (
    <article className="mx-auto grid w-full place-items-center">
      <div className="prose dark:prose-invert">
        <ReactMarkdown>{doc.content}</ReactMarkdown>
      </div>
    </article>
  );
}
