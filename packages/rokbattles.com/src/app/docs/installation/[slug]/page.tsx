import { notFound } from "next/navigation";
import { DownloadOptions } from "@/components/docs/download-options";
import { MarkdownDocument } from "@/components/markdown-document";
import Android from "@/content/docs/installation/android.mdx";
import IOS from "@/content/docs/installation/ios.mdx";
import Linux from "@/content/docs/installation/linux.mdx";
import MacOS from "@/content/docs/installation/macos.mdx";
import SteamOS from "@/content/docs/installation/steamos.mdx";
import Windows from "@/content/docs/installation/windows.mdx";
import { installationDocs } from "@/content/metadata";

const documents = {
  android: Android,
  ios: IOS,
  linux: Linux,
  macos: MacOS,
  steamos: SteamOS,
  windows: Windows,
};
const components = { DownloadOptions };

export const dynamicParams = false;

export function generateStaticParams() {
  return installationDocs.map(({ slug }) => ({ slug }));
}

function getDocument(slug: string) {
  const document = installationDocs.find((item) => item.slug === slug);
  if (!document || !(slug in documents)) notFound();

  return { ...document, Content: documents[slug as keyof typeof documents] };
}

export async function generateMetadata({ params }: PageProps<"/docs/installation/[slug]">) {
  const { slug } = await params;
  const document = getDocument(slug);

  return {
    title: `${document.title} installation`,
    description: document.description,
  };
}

export default async function InstallationPage({ params }: PageProps<"/docs/installation/[slug]">) {
  const { slug } = await params;
  const document = getDocument(slug);

  return (
    <MarkdownDocument Content={document.Content} title={document.title} components={components} />
  );
}
