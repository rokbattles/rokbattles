import { MongoClient } from "mongodb";

if (!process.env.MONGO_URI) {
  throw new Error("Invalid/missing environment variable: MONGO_URI");
}

let client: MongoClient;

if (process.env.NODE_ENV === "development") {
  // In development mode, use a global variable so that the value
  // is preserved across module reloads caused by HMR (Hot Module Replacement).
  const globalWithMongo = global as typeof globalThis & {
    _mongoClient?: MongoClient;
  };

  if (!globalWithMongo._mongoClient) {
    globalWithMongo._mongoClient = new MongoClient(process.env.MONGO_URI, {
      appName: "ROK Battles",
    });
  }
  client = globalWithMongo._mongoClient;
} else {
  client = new MongoClient(process.env.MONGO_URI, { appName: "ROK Battles" });
}

export default client;

export function toPlainObject(source: unknown): Record<string, unknown> {
  if (!source || typeof source !== "object") {
    return {};
  }

  const serialized = JSON.stringify(source, (_, value) => {
    if (
      value &&
      typeof value === "object" &&
      "$numberLong" in (value as Record<string, unknown>) &&
      typeof (value as Record<string, unknown>).$numberLong === "string"
    ) {
      const numericValue = Number((value as { $numberLong: string }).$numberLong);
      return Number.isFinite(numericValue)
        ? numericValue
        : (value as { $numberLong: string }).$numberLong;
    }
    return value;
  });

  return JSON.parse(serialized) as Record<string, unknown>;
}
