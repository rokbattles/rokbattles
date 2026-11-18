import { redirect } from "next/navigation";

export default async function Page({ params }: PageProps<"/developer/apps/[id]/scopes">) {
  const { id } = await params;
  redirect(`/developer/apps/${encodeURIComponent(id)}/authorization`);
}
