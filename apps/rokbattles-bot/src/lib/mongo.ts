import { MongoClient } from "mongodb";

if (!process.env.MONGO_URI) {
  console.error("MONGO_URI must be set");
  process.exit(1);
}

let _mongo: MongoClient | null = null;

export async function mongo(): Promise<MongoClient> {
  if (!_mongo) {
    _mongo = new MongoClient(process.env.MONGO_URI, {
      appName: "ROK Battles Bot",
    });
    await _mongo.connect();
  }

  return _mongo;
}
