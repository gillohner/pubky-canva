"use client";

import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useAuthStore } from "@/stores/auth-store";
import { placePixel } from "@/lib/pubky/pixels";
import type { CanvasResponse, PixelState } from "@/types/canvas";

interface PlacePixelArgs {
  x: number;
  y: number;
  color: number;
}

export function usePlacePixel() {
  const queryClient = useQueryClient();
  const session = useAuthStore((s) => s.session);
  const publicKey = useAuthStore((s) => s.publicKey);

  return useMutation({
    mutationFn: async ({ x, y, color }: PlacePixelArgs) => {
      if (!session) throw new Error("Not authenticated");
      return placePixel(session, x, y, color);
    },
    onMutate: async ({ x, y, color }) => {
      // Optimistic update
      await queryClient.cancelQueries({ queryKey: ["canvas"] });
      const previous = queryClient.getQueryData<CanvasResponse>(["canvas"]);

      if (previous && publicKey) {
        const optimistic: PixelState = {
          x,
          y,
          color,
          user_pk: publicKey,
          placed_at: Date.now() * 1000, // microseconds
        };

        const pixels = previous.pixels.filter(
          (p) => !(p.x === x && p.y === y)
        );
        pixels.push(optimistic);

        queryClient.setQueryData<CanvasResponse>(["canvas"], {
          ...previous,
          pixels,
        });
      }

      return { previous };
    },
    onError: (_err, _vars, context) => {
      // Rollback
      if (context?.previous) {
        queryClient.setQueryData(["canvas"], context.previous);
      }
    },
    onSettled: () => {
      // Invalidate credits
      if (publicKey) {
        queryClient.invalidateQueries({
          queryKey: ["credits", publicKey],
        });
      }
    },
  });
}
