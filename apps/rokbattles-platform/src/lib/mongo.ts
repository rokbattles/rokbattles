import { MongoClient, type MongoClientOptions } from "mongodb";

const uri = process.env.MONGO_URI;
const options: MongoClientOptions = {
  appName: "ROK Battles",
};

let client: MongoClient;
let clientPromise: Promise<MongoClient> | null = null;

if (uri) {
  if (process.env.NODE_ENV === "development") {
    // In development mode, use a global variable so that the value
    // is preserved across module reloads caused by HMR (Hot Module Replacement).
    const globalWithMongo = global as typeof globalThis & {
      _mongoClientPromise?: Promise<MongoClient>;
    };

    if (!globalWithMongo._mongoClientPromise) {
      client = new MongoClient(uri, options);
      globalWithMongo._mongoClientPromise = client.connect();
    }
    clientPromise = globalWithMongo._mongoClientPromise;
  } else {
    // In production mode, it's best to not use a global variable.
    client = new MongoClient(uri, options);
    clientPromise = client.connect();
  }
}

export default clientPromise;

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
      const numericValue = Number(
        (value as { $numberLong: string }).$numberLong
      );
      return Number.isFinite(numericValue)
        ? numericValue
        : (value as { $numberLong: string }).$numberLong;
    }
    return value;
  });

  return JSON.parse(serialized) as Record<string, unknown>;
}
