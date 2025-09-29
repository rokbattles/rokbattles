"use client";

import { newHttpBatchRpcSession, type RpcStub } from "capnweb";
import { useEffect, useRef, useState } from "react";
import type { PublicApi } from "@/lib/rpc/types";

export default function Page() {
  const [data, setData] = useState<string | null>(null);
  const ref = useRef<boolean>(false);

  useEffect(() => {
    if (ref.current) return;
    ref.current = true;

    const controller = new AbortController();
    const signal = controller.signal;

    (async () => {
      try {
        const api: RpcStub<PublicApi> = newHttpBatchRpcSession<PublicApi>("/api/rpc", { signal });
        const result = await api.test();

        if (!signal.aborted) {
          setData(result);
        }
      } catch (err) {
        if (err.name !== "AbortError") {
          console.error(err);
        }
      }
    })();

    return () => {
      controller.abort();
    };
  }, []);

  if (!data) {
    return <p>Loading</p>;
  }

  return <p>Result: {data}</p>;
}
