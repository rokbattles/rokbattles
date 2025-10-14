import type { Metadata } from "next";
import { notFound } from "next/navigation";
import ReactMarkdown from "react-markdown";
import { getLegalDocument, getLegalDocuments, loadLegalDocument } from "@/lib/legal-docs";

export function generateStaticParams() {
  return getLegalDocuments().map((doc) => ({ slug: doc.slug }));
}

export async function generateMetadata({ params }: PageProps<"/legal/[slug]">): Promise<Metadata> {
  const { slug } = await params;
  const doc = getLegalDocument(slug);

  return {
    title: doc.title,
  };
}

export default async function LegalDocumentPage({ params }: PageProps<"/legal/[slug]">) {
  const { slug } = await params;
  const doc = await loadLegalDocument(slug);

  if (!doc) {
    notFound();
  }

  return (
    <div className="relative min-h-dvh bg-zinc-950 text-zinc-100 antialiased">
      <article className="mx-auto grid min-h-dvh max-w-7xl place-items-center px-6 py-12 md:py-20">
        <div className="prose prose-invert">
          <ReactMarkdown>{doc.content}</ReactMarkdown>
        </div>
      </article>
    </div>
  );
}
