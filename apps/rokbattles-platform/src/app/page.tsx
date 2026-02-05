import { useExtracted } from "next-intl";

export default function Page() {
  const t = useExtracted();

  return <div>{t("ROK Battles")}</div>;
}
