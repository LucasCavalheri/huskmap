import { en, type Dictionary } from "./en";
import { ptBR } from "./pt-BR";

export type Locale = "en" | "pt-BR";
export type { Dictionary };

export const dictionaries: Record<Locale, Dictionary> = { en, "pt-BR": ptBR };

/** English owns `/`; Portuguese lives under `/pt-br/`. */
export const localePath = (locale: Locale): string => (locale === "en" ? "/" : "/pt-br/");
export const otherLocale = (locale: Locale): Locale => (locale === "en" ? "pt-BR" : "en");
