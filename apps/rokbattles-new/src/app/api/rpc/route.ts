import { newHttpBatchRpcResponse } from "capnweb";
import type { NextRequest } from "next/server";
import { PublicApiImpl } from "@/lib/rpc/server";

export async function POST(req: NextRequest) {
  return await newHttpBatchRpcResponse(req, new PublicApiImpl());
}
