import { board, resolveRelease } from "./release.mjs";

export interface DownloadFile {
  name: string;
  url: string;
  size: number;
}
export interface Downloads {
  version: string;
  page: string;
  sums: string | null;
  files: Record<string, Record<string, DownloadFile | null>>;
}

const env = (globalThis as { process?: { env?: Record<string, string | undefined> } }).process?.env ?? {};
const resolve = resolveRelease as (env: Record<string, string | undefined>) => Promise<unknown | null>;
const toBoard = board as (release: unknown) => Downloads;

const latest = await resolve(env);

export const downloads: Downloads | null = latest ? toBoard(latest) : null;
