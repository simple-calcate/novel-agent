import { createContext, useContext } from "react";
import type { StoryEntry } from "../types";

export interface StoryCatalogValue {
  entries: StoryEntry[];
  nearby: string;
}

export const StoryCatalogContext = createContext<StoryCatalogValue>({
  entries: [],
  nearby: "",
});

export function useStoryCatalog(): StoryCatalogValue {
  return useContext(StoryCatalogContext);
}
