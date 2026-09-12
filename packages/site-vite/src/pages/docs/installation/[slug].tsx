import { Suspense } from "react";
import { DownloadOptions } from "../../../components/docs/download-options";
import { MarkdownDocument } from "../../../components/markdown-document";
import Metadata from "../../../components/metadata";
import { Text } from "../../../components/ui/marketing/text";
import type { installationDocs } from "../../../lib/docs";

const components = { DownloadOptions };

export default function InstallationRoute({
  document,
}: {
  document: (typeof installationDocs)[number];
}) {
  return (
    <>
      <Metadata title={`${document.title} installation`} description={document.description} />
      <Suspense
        key={document.slug}
        fallback={<Text role="status">Loading installation guide...</Text>}
      >
        <MarkdownDocument
          title={document.title}
          Content={document.Content}
          components={components}
        />
      </Suspense>
    </>
  );
}
