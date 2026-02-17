"use client";

import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useAuthStore } from "@/stores/auth-store";
import { placePixel } from "@/lib/pubky/pixels";
import type {
  CanvasResponse,
  CreditsResponse,
  PixelState,
} from "@/types/canvas";

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
      // Cancel in-flight queries so they don't overwrite our optimistic state
      await queryClient.cancelQueries({ queryKey: ["canvas"] });
      if (publicKey) {
        await queryClient.cancelQueries({
          queryKey: ["credits", publicKey],
        });
      }

      // Snapshot previous state for rollback
      const previousCanvas =
        queryClient.getQueryData<CanvasResponse>(["canvas"]);
      const previousCredits = publicKey
        ? queryClient.getQueryData<CreditsResponse>(["credits", publicKey])
        : undefined;

      // Optimistic canvas update
      if (previousCanvas && publicKey) {
        const optimistic: PixelState = {
          x,
          y,
          color,
          user_pk: publicKey,
          placed_at: Date.now() * 1000, // microseconds
        };

        const pixels = previousCanvas.pixels.filter(
          (p) => !(p.x === x && p.y === y)
        );
        pixels.push(optimistic);

        queryClient.setQueryData<CanvasResponse>(["canvas"], {
          ...previousCanvas,
          pixels,
        });
      }

      // Optimistic credits decrement
      if (previousCredits && publicKey) {
        queryClient.setQueryData<CreditsResponse>(["credits", publicKey], {
          ...previousCredits,
          credits: Math.max(0, previousCredits.credits - 1),
          // If we were at max, regen timer starts now (~full interval)
          next_credit_in_seconds:
            previousCredits.credits >= previousCredits.max_credits
              ? previousCredits.next_credit_in_seconds ?? 600
              : previousCredits.next_credit_in_seconds,
        });
      }

      return { previousCanvas, previousCredits };
    },
    onError: (_err, _vars, context) => {
      // Rollback both canvas and credits
      if (context?.previousCanvas) {
        queryClient.setQueryData(["canvas"], context.previousCanvas);
      }
      if (context?.previousCredits && publicKey) {
        queryClient.setQueryData(
          ["credits", publicKey],
          context.previousCredits
        );
      }
    },
    onSettled: () => {
      // Reconcile with server truth
      if (publicKey) {
        queryClient.invalidateQueries({
          queryKey: ["credits", publicKey],
        });
      }
    },
  });
}
