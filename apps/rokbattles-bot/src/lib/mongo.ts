import { MongoClient } from "mongodb";

let _mongo: MongoClient | null = null;

export async function mongo(): Promise<MongoClient> {
  if (!_mongo) {
    // biome-ignore lint/style/noNonNullAssertion: ignore
    _mongo = new MongoClient(process.env.MONGO_URI!, {
      appName: "ROK Battles Bot",
    });
    await _mongo.connect();
  }

  return _mongo;
}
