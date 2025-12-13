import { NextResponse } from "next/server";

export function notImplementedResponse() {
  return NextResponse.json({ error: "Not implemented yet" }, { status: 405 });
}
