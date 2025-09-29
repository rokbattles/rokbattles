import { RpcTarget } from "capnweb";
import type { PublicApi } from "./types";

export class PublicApiImpl extends RpcTarget implements PublicApi {
  test() {
    return "Hello world!";
  }
}
