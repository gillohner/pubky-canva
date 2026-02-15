"use client";

import { useQuery } from "@tanstack/react-query";
import { fetchCanvas, fetchMeta } from "@/lib/api/canvas";

export function useCanvas() {
  return useQuery({
    queryKey: ["canvas"],
    queryFn: fetchCanvas,
    refetchInterval: 30000, // Fallback polling every 30s
  });
}

export function useCanvasMeta() {
  return useQuery({
    queryKey: ["canvas-meta"],
    queryFn: fetchMeta,
    refetchInterval: 10000,
  });
}
