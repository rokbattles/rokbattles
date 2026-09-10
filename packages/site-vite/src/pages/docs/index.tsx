import { lazy, Suspense } from "react";
import { MarkdownDocument } from "../../components/markdown-document";
import Metadata from "../../components/metadata";
import { Text } from "../../components/ui/marketing/text";

const GettingStarted = lazy(() => import("../../content/docs/getting-started.md"));

export default function DocsRoute() {
  return (
    <>
      <Metadata
        title="Docs"
        description="Download and install ROK Battles on desktop and mobile. Choose a platform to get started."
      />
      <Suspense fallback={<Text role="status">Loading documentation...</Text>}>
        <MarkdownDocument Content={GettingStarted} title="Get started with ROK Battles" />
      </Suspense>
    </>
  );
}
