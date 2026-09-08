import { Fragment } from "react";
import Metadata from "../components/metadata";
import { Heading } from "../components/ui/heading";

export default function IndexRoute() {
  return (
    <Fragment>
      <Metadata />
      <Heading>ROK Battles</Heading>
    </Fragment>
  );
}
