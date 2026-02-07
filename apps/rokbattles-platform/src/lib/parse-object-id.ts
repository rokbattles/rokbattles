import { ObjectId } from "mongodb";

export function parseObjectId(value: string | undefined): ObjectId | null {
  if (!(value && ObjectId.isValid(value))) {
    return null;
  }

  return new ObjectId(value);
}
