import { cn } from "cn";
import { ArrowLeft } from "lucide-react";
import { notFound } from "next/navigation";
import { MarkdownDocument } from "@/components/(marketing)/markdown-document";
import { gutter } from "@/components/(marketing)/ui/layout";
import { Link } from "@/components/(marketing)/ui/link";
import CookiePolicy from "@/content/legal/cookie-policy.md";
import PrivacyPolicy from "@/content/legal/privacy-policy.md";
import TermsOfService from "@/content/legal/terms-of-service.md";
import { legalDocuments } from "@/content/metadata";

const documents = {
  "cookie-policy": CookiePolicy,
  "privacy-policy": PrivacyPolicy,
  "terms-of-service": TermsOfService,
};

export const dynamicParams = false;

export function generateStaticParams() {
  return legalDocuments.map(({ id }) => ({ slug: id }));
}

function getDocument(slug: string) {
  const document = legalDocuments.find((item) => item.id === slug);
  if (!document || !(slug in documents)) notFound();

  return { ...document, Content: documents[slug as keyof typeof documents] };
}

export async function generateMetadata({ params }: PageProps<"/legal/[slug]">) {
  const { slug } = await params;
  const document = getDocument(slug);

  return {
    title: document.title,
    description: document.description,
  };
}

export default async function LegalDocumentPage({ params }: PageProps<"/legal/[slug]">) {
  const { slug } = await params;
  const document = getDocument(slug);

  return (
    <div className={cn(gutter, "border-b border-white/10 py-10 sm:py-12")}>
      <div className="mx-auto max-w-3xl">
        <Link href="/legal" className="mb-8">
          <ArrowLeft aria-hidden="true" /> All legal documents
        </Link>
        <MarkdownDocument Content={document.Content} title={document.title} />
      </div>
    </div>
  );
}
