import { Fragment } from "react";

interface Props {
  title?: string;
  description?: string;
}

export default function Metadata({ title, description }: Props) {
  const pageTitle = title ? `${title} | ROK Battles` : "ROK Battles";
  const pageDescription =
    description ||
    "A community-driven platform for sharing battle reports and surfacing actionable trends in Rise of Kingdoms";

  return (
    <Fragment>
      <title>{pageTitle}</title>
      <meta name="description" content={pageDescription} />
      <meta property="og:title" content={pageTitle} />
      <meta property="og:description" content={pageDescription} />
    </Fragment>
  );
}
