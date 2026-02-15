"use client";

import { useMemo } from "react";
import { useQuery } from "@tanstack/react-query";
import { fetchPixelInfo } from "@/lib/api/canvas";
import { useProfileMap } from "@/hooks/use-profile";
import { PICO8_PALETTE, PICO8_NAMES } from "@/types/canvas";
import { config } from "@/lib/config";
import { X, ExternalLink } from "lucide-react";

interface PixelInfoProps {
  x: number;
  y: number;
  onClose: () => void;
}

function truncatePk(pk: string): string {
  return `${pk.slice(0, 10)}...${pk.slice(-6)}`;
}

function timeAgo(microseconds: number): string {
  const seconds = Math.floor((Date.now() * 1000 - microseconds) / 1_000_000);
  if (seconds < 60) return `${seconds}s ago`;
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.floor(hours / 24);
  return `${days}d ago`;
}

export function PixelInfoPanel({ x, y, onClose }: PixelInfoProps) {
  const { data, isLoading } = useQuery({
    queryKey: ["pixel-info", x, y],
    queryFn: () => fetchPixelInfo(x, y),
  });

  const userPks = useMemo(() => {
    if (!data) return [];
    const set = new Set<string>();
    set.add(data.current.user_pk);
    data.history.forEach((e) => set.add(e.user_pk));
    return Array.from(set);
  }, [data]);

  const profileMap = useProfileMap(userPks);

  function displayName(pk: string): string {
    return profileMap.get(pk) || truncatePk(pk);
  }

  return (
    <div className="w-64 rounded-xl border border-neutral-700 bg-neutral-900 p-4 shadow-2xl">
      <div className="mb-3 flex items-center justify-between">
        <span className="text-sm font-medium text-white">
          Pixel ({x}, {y})
        </span>
        <button
          onClick={onClose}
          className="text-neutral-400 hover:text-white"
        >
          <X size={14} />
        </button>
      </div>

      {isLoading && (
        <p className="text-xs text-neutral-500">Loading...</p>
      )}

      {data && (
        <div className="space-y-3">
          <div className="flex items-center gap-2">
            <div
              className="h-6 w-6 rounded border border-neutral-600"
              style={{
                backgroundColor: PICO8_PALETTE[data.current.color],
              }}
            />
            <span className="text-sm text-neutral-300">
              {PICO8_NAMES[data.current.color]}
            </span>
          </div>

          <div>
            <div className="mb-1 text-xs text-neutral-500">Placed by</div>
            <a
              href={`${config.pubkyApp.profileUrl}/${data.current.user_pk}`}
              target="_blank"
              rel="noopener noreferrer"
              className="flex items-center gap-1 text-sm text-blue-400 hover:text-blue-300"
            >
              {displayName(data.current.user_pk)}
              <ExternalLink size={12} />
            </a>
            <div className="text-xs text-neutral-500">
              {timeAgo(data.current.placed_at)}
            </div>
          </div>

          {data.history.length > 1 && (
            <div>
              <div className="mb-1 text-xs text-neutral-500">History</div>
              <div className="max-h-32 space-y-1 overflow-y-auto">
                {data.history.map((entry) => (
                  <div
                    key={entry.id}
                    className="flex items-center gap-2 text-xs"
                  >
                    <div
                      className="h-3 w-3 rounded"
                      style={{
                        backgroundColor: PICO8_PALETTE[entry.color],
                      }}
                    />
                    <a
                      href={`${config.pubkyApp.profileUrl}/${entry.user_pk}`}
                      target="_blank"
                      rel="noopener noreferrer"
                      className="text-neutral-400 hover:text-blue-400"
                    >
                      {displayName(entry.user_pk)}
                    </a>
                    <span className="text-neutral-600">
                      {timeAgo(entry.placed_at)}
                    </span>
                  </div>
                ))}
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
