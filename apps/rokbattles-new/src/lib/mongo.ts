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
