import { findSiblings } from "fumadocs-core/page-tree";
import { Card, Cards } from "fumadocs-ui/components/card";
import { source } from "@/lib/source";

export function DocsCategory({ url }: { url: string }) {
  return (
    <Cards>
      {findSiblings(source.getPageTree(), url).map((item) => {
        if (item.type === "separator") return null;
        if (item.type === "folder") {
          if (!item.index) return null;
          item = item.index;
        }

        return (
          <Card key={item.url} title={item.name} href={item.url}>
            {item.description}
          </Card>
        );
      })}
    </Cards>
  );
}
