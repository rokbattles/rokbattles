import { DocsLayout } from "fumadocs-ui/layouts/notebook";
import { source } from "@/lib/source";

export default function Layout({ children }: LayoutProps<"/docs">) {
  return (
    <DocsLayout tree={source.getPageTree()} nav={{ title: "ROK Battles" }}>
      {children}
    </DocsLayout>
  );
}
