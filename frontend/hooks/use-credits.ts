"use client";

import { useQuery } from "@tanstack/react-query";
import { useAuthStore } from "@/stores/auth-store";
import { fetchCredits } from "@/lib/api/canvas";

export function useCredits() {
  const publicKey = useAuthStore((s) => s.publicKey);
  const isAuthenticated = useAuthStore((s) => s.isAuthenticated);

  return useQuery({
    queryKey: ["credits", publicKey],
    queryFn: () => fetchCredits(publicKey!),
    enabled: isAuthenticated && !!publicKey,
    refetchInterval: 10000, // Poll every 10s to update credit regen
  });
}
