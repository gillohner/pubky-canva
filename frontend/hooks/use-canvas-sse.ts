"use client";

import { useEffect } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { config } from "@/lib/config";
import type { PixelState, CanvasResponse } from "@/types/canvas";

export function useCanvasSSE() {
  const queryClient = useQueryClient();

  useEffect(() => {
    const es = new EventSource(`${config.indexerUrl}/api/events`);

    es.addEventListener("pixel", (e) => {
      try {
        const pixel: PixelState = JSON.parse(e.data);

        // Update canvas query data in-place
        queryClient.setQueryData<CanvasResponse>(["canvas"], (old) => {
          if (!old) return old;

          const pixels = old.pixels.filter(
            (p) => !(p.x === pixel.x && p.y === pixel.y)
          );
          pixels.push(pixel);

          return { ...old, pixels };
        });

        // Invalidate meta (fill stats changed)
        queryClient.invalidateQueries({ queryKey: ["canvas-meta"] });
      } catch {
        // ignore parse errors
      }
    });

    es.addEventListener("resize", () => {
      // Refetch everything on resize
      queryClient.invalidateQueries({ queryKey: ["canvas"] });
      queryClient.invalidateQueries({ queryKey: ["canvas-meta"] });
    });

    es.onerror = () => {
      // EventSource auto-reconnects
    };

    return () => es.close();
  }, [queryClient]);
}
