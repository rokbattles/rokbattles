export const RESOURCE_CATALOG = [
  { key: "type:1", type: 1, color: "#16a34a" },
  { key: "type:2", type: 2, color: "#d97706" },
  { key: "type:3", type: 3, color: "#8b8fa8" },
  { key: "type:4", type: 4, color: "#ca8a04" },
  { key: "type:5", type: 5, color: "#f43f5e" },
  { key: "crystalsGain", type: 9, color: "#0ea5e9" },
] as const;

export type ResourceKey = (typeof RESOURCE_CATALOG)[number]["key"];

export const RESOURCE_KEYS = RESOURCE_CATALOG.map((resource) => resource.key);
