export interface PixelState {
  x: number;
  y: number;
  color: number;
  user_pk: string;
  placed_at: number;
}

export interface CanvasResponse {
  size: number;
  pixels: PixelState[];
}

export interface CanvasMeta {
  size: number;
  total_pixels: number;
  filled: number;
  overwritten: number;
  max_credits: number;
  credit_regen_seconds: number;
}

export interface PixelHistoryEntry {
  id: string;
  user_pk: string;
  color: number;
  placed_at: number;
}

export interface PixelInfo {
  current: PixelState;
  history: PixelHistoryEntry[];
}

export interface CreditsResponse {
  credits: number;
  max_credits: number;
  next_credit_in_seconds: number | null;
}

export interface SsePixelEvent {
  type: "pixel";
  x: number;
  y: number;
  color: number;
  user_pk: string;
  placed_at: number;
}

export interface SseResizeEvent {
  type: "resize";
  old_size: number;
  new_size: number;
}

export const PICO8_PALETTE = [
  "#000000", // 0: Black
  "#1D2B53", // 1: Dark Blue
  "#7E2553", // 2: Dark Purple
  "#008751", // 3: Dark Green
  "#AB5236", // 4: Brown
  "#5F574F", // 5: Dark Grey
  "#C2C3C7", // 6: Light Grey
  "#FFF1E8", // 7: White
  "#FF004D", // 8: Red
  "#FFA300", // 9: Orange
  "#FFEC27", // 10: Yellow
  "#00E436", // 11: Green
  "#29ADFF", // 12: Blue
  "#83769C", // 13: Lavender
  "#FF77A8", // 14: Pink
  "#FFCCAA", // 15: Peach
] as const;

export const PICO8_NAMES = [
  "Black", "Dark Blue", "Dark Purple", "Dark Green",
  "Brown", "Dark Grey", "Light Grey", "White",
  "Red", "Orange", "Yellow", "Green",
  "Blue", "Lavender", "Pink", "Peach",
] as const;
