import { getExtracted } from "next-intl/server";

export default async function Page() {
  const t = await getExtracted();

  return <div>{t("ROK Battles")}</div>;
}
