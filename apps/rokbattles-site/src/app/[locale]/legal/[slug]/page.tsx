import type { Metadata } from "next";
import { notFound } from "next/navigation";
import type { Locale } from "next-intl";
import { hasLocale } from "next-intl";
import { setRequestLocale } from "next-intl/server";
import ReactMarkdown from "react-markdown";
import { routing } from "@/i18n/routing";
import { getLegalDocument, getLegalDocuments, loadLegalDocument } from "@/lib/legal-docs";

export const dynamic = "error";
export const dynamicParams = false;

export function generateStaticParams() {
  return routing.locales.flatMap((locale) =>
    getLegalDocuments().map((doc) => ({ locale, slug: doc.slug }))
  );
}

export async function generateMetadata({
  params,
}: PageProps<"/[locale]/legal/[slug]">): Promise<Metadata> {
  const { slug } = await params;
  const doc = getLegalDocument(slug);

  return {
    title: doc.title,
  };
}

export default async function LegalDocumentPage({ params }: PageProps<"/[locale]/legal/[slug]">) {
  const { locale: rawLocale, slug } = await params;
  const resolvedLocale = hasLocale(routing.locales, rawLocale) ? rawLocale : routing.defaultLocale;
  const locale: Locale = resolvedLocale;

  setRequestLocale(locale);

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
