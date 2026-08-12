"use client";

import { useContext, useEffect } from "react";
import { GameTranslationsContext } from "@/providers/game-translation-context";

export function GameTranslate({ value }: { value: string }) {
  const context = useContext(GameTranslationsContext);

  useEffect(() => context?.registerKey(value), [context?.registerKey, value]);

  return context?.translations[value] ?? value;
}
