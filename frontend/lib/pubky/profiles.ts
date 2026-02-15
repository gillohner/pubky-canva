import { Pubky, type Address } from "@synonymdev/pubky";

export interface PubkyProfile {
  name: string;
  bio?: string;
  image?: string;
}

let pubkyInstance: Pubky | null = null;

function getPubky(): Pubky {
  if (!pubkyInstance) {
    pubkyInstance = new Pubky();
  }
  return pubkyInstance;
}

export async function fetchProfile(pk: string): Promise<PubkyProfile | null> {
  try {
    const pubky = getPubky();
    const addr = `${pk}/pub/pubky.app/profile.json` as Address;
    const response = await pubky.publicStorage.get(addr);
    const text = await response.text();
    const data = JSON.parse(text);
    if (data && typeof data === "object" && "name" in data) {
      return data as PubkyProfile;
    }
    return null;
  } catch (e) {
    console.warn(`[pubky-canva] Failed to fetch profile for ${pk}:`, e);
    return null;
  }
}
