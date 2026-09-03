import { getGameSpriteUrl } from "../game-sprite";
import type { BuildingKind, LandmarkKind, ResourceKind } from "./types";

export type SpriteCrop = {
  x: number;
  y: number;
  width: number;
  height: number;
};

export const BUILDING_SPRITES: Record<BuildingKind, string> = {
  flag: getGameSpriteUrl("img_iconAlliFlag.png"),
  mainFortress: getGameSpriteUrl("img_iconAlliMainHall.png"),
  subFortress: getGameSpriteUrl("img_iconAlliMainHall.png"),
  horse: getGameSpriteUrl("img_iconAlliMainMuMa2.png"),
};

export const BUILDING_SPRITE_CROPS: Record<BuildingKind, SpriteCrop> = {
  flag: { x: 55, y: 46, width: 128, height: 187 },
  mainFortress: { x: 48, y: 55, width: 168, height: 185 },
  subFortress: { x: 48, y: 55, width: 168, height: 185 },
  horse: { x: 56, y: 26, width: 128, height: 206 },
};

export const RESOURCE_SPRITES: Record<ResourceKind | "credits", string> = {
  food: getGameSpriteUrl("img_Allifood.png"),
  wood: getGameSpriteUrl("img_Alliwood.png"),
  stone: getGameSpriteUrl("img_Allistone.png"),
  coin: getGameSpriteUrl("img_Alligold.png"),
  crystal: getGameSpriteUrl("img_AlliCrystal.png"),
  credits: getGameSpriteUrl("img_iconAllianceFund.png"),
};

export const LANDMARK_SPRITES: Record<LandmarkKind, string> = {
  village: getGameSpriteUrl("village_lod3.png"),
  cave: getGameSpriteUrl("cave_lod3.png"),
};
