"use client";

import { useState, useMemo, useCallback } from "react";
import { useCanvas } from "@/hooks/use-canvas";
import { useCanvasSSE } from "@/hooks/use-canvas-sse";
import { usePlacePixel } from "@/hooks/use-place-pixel";
import { useCredits } from "@/hooks/use-credits";
import { useProfileMap } from "@/hooks/use-profile";
import { useAuthStore } from "@/stores/auth-store";
import { PICO8_PALETTE } from "@/types/canvas";
import { ColorPicker } from "./color-picker";
import { PixelInfoPanel } from "./pixel-info";
import { toast } from "sonner";

type SelectedPixel = { x: number; y: number };

export function PixelCanvas() {
  const { data, isLoading } = useCanvas();
  const { data: credits } = useCredits();
  const isAuthenticated = useAuthStore((s) => s.isAuthenticated);
  const placePixel = usePlacePixel();

  const [selected, setSelected] = useState<SelectedPixel | null>(null);
  const [showInfo, setShowInfo] = useState<SelectedPixel | null>(null);
  const [showPicker, setShowPicker] = useState(false);

  // Subscribe to SSE updates
  useCanvasSSE();

  // Build pixel lookup map
  const pixelMap = useMemo(() => {
    if (!data) return new Map<string, { color: number; user_pk: string }>();
    const map = new Map<string, { color: number; user_pk: string }>();
    for (const p of data.pixels) {
      map.set(`${p.x},${p.y}`, { color: p.color, user_pk: p.user_pk });
    }
    return map;
  }, [data]);

  const uniqueUserPks = useMemo(() => {
    if (!data) return [];
    const set = new Set<string>();
    for (const p of data.pixels) set.add(p.user_pk);
    return Array.from(set);
  }, [data]);

  const profileMap = useProfileMap(uniqueUserPks);

  const width = data?.width ?? 16;
  const height = data?.height ?? 16;

  const handleCellClick = useCallback(
    (x: number, y: number) => {
      const pixel = pixelMap.get(`${x},${y}`);

      if (isAuthenticated) {
        // Authenticated: show color picker + info if pixel has data
        setSelected({ x, y });
        setShowPicker(true);
        setShowInfo(pixel ? { x, y } : null);
      } else if (pixel) {
        // Not authenticated: show info for filled pixel
        setShowInfo({ x, y });
        setShowPicker(false);
        setSelected(null);
      }
    },
    [isAuthenticated, pixelMap]
  );

  const handleColorSelect = useCallback(
    (colorIndex: number) => {
      if (!selected) return;

      if (credits && credits.credits <= 0) {
        toast.error("No credits available! Wait for regeneration.");
        return;
      }

      placePixel.mutate(
        { x: selected.x, y: selected.y, color: colorIndex },
        {
          onSuccess: () => {
            toast.success(
              `Pixel placed at (${selected.x}, ${selected.y})`
            );
          },
          onError: (e) => {
            toast.error(`Failed: ${e.message}`);
          },
        }
      );

      setShowPicker(false);
      setSelected(null);
    },
    [selected, credits, placePixel]
  );

  const handleCancel = useCallback(() => {
    setShowPicker(false);
    setSelected(null);
    setShowInfo(null);
  }, []);

  if (isLoading) {
    return (
      <div className="flex h-full items-center justify-center">
        <div className="text-neutral-500">Loading canvas...</div>
      </div>
    );
  }

  // Calculate cell size to fit viewport (constrained by whichever dimension is tighter)
  const cellSize = `min(calc((100vw - 2rem) / ${width}), calc((100vh - 8rem) / ${height}))`;

  return (
    <div className="relative flex flex-col items-center gap-4">
      {/* Canvas grid */}
      <div
        className="relative grid border border-neutral-700"
        style={{
          gridTemplateColumns: `repeat(${width}, ${cellSize})`,
          gridTemplateRows: `repeat(${height}, ${cellSize})`,
        }}
      >
        {Array.from({ length: width * height }, (_, i) => {
          const x = i % width;
          const y = Math.floor(i / width);
          const pixel = pixelMap.get(`${x},${y}`);
          const isSelected =
            selected?.x === x && selected?.y === y;

          return (
            <div
              key={`${x},${y}`}
              onClick={() => handleCellClick(x, y)}
              className={`cursor-pointer border border-neutral-800/50 transition-all hover:brightness-125 ${
                isSelected ? "ring-2 ring-white ring-offset-1 ring-offset-neutral-950" : ""
              }`}
              style={{
                backgroundColor: pixel
                  ? PICO8_PALETTE[pixel.color]
                  : "#111111",
              }}
              title={
                pixel
                  ? `(${x},${y}) ${profileMap.get(pixel.user_pk) || pixel.user_pk.slice(0, 8) + "..."}`
                  : `(${x},${y}) empty`
              }
            />
          );
        })}
        {/* Popover positioned next to the selected pixel */}
        {(selected || showInfo) && (showPicker || showInfo) && (() => {
          const activePixel = selected || showInfo!;
          const isLeftHalf = activePixel.x < width / 2;
          return (
            <div
              className="absolute z-10"
              style={{
                top: `calc(${((activePixel.y + 0.5) / height) * 100}%)`,
                ...(isLeftHalf
                  ? { left: `calc(${((activePixel.x + 1) / width) * 100}% + 8px)` }
                  : { right: `calc(${((width - activePixel.x) / width) * 100}% + 8px)` }),
                transform: 'translateY(-50%)',
              }}
            >
              <div className="flex flex-col gap-2">
                {showInfo && (
                  <PixelInfoPanel
                    x={showInfo.x}
                    y={showInfo.y}
                    onClose={handleCancel}
                  />
                )}
                {showPicker && selected && (
                  <ColorPicker
                    onSelect={handleColorSelect}
                    onCancel={handleCancel}
                    disabled={placePixel.isPending}
                  />
                )}
              </div>
            </div>
          );
        })()}
      </div>
    </div>
  );
}
