import { ExploreLayout } from "@/components/explore/ExploreLayout";

export default function Layout({ children }: LayoutProps<"/[locale]/explore">) {
  return <ExploreLayout>{children}</ExploreLayout>;
}
