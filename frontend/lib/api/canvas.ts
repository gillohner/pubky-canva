import { apiFetch } from "./client";
import type {
  CanvasResponse,
  CanvasMeta,
  PixelInfo,
  CreditsResponse,
} from "@/types/canvas";

export function fetchCanvas(): Promise<CanvasResponse> {
  return apiFetch<CanvasResponse>("/api/canvas");
}

export function fetchMeta(): Promise<CanvasMeta> {
  return apiFetch<CanvasMeta>("/api/canvas/meta");
}

export function fetchPixelInfo(x: number, y: number): Promise<PixelInfo> {
  return apiFetch<PixelInfo>(`/api/canvas/pixel/${x}/${y}`);
}

export function fetchCredits(publicKey: string): Promise<CreditsResponse> {
  return apiFetch<CreditsResponse>(`/api/user/${publicKey}/credits`);
}

export function ingestUser(publicKey: string): Promise<void> {
  return apiFetch(`/api/ingest/${publicKey}`, { method: "PUT" });
}

export interface PubkyProfile {
  name: string;
  bio?: string;
  image?: string;
}

export function fetchProfile(publicKey: string): Promise<PubkyProfile> {
  return apiFetch<PubkyProfile>(`/api/user/${publicKey}/profile`);
}
