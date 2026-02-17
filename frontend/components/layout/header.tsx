"use client";

import { useState } from "react";
import { useAuthStore } from "@/stores/auth-store";
import { useCredits } from "@/hooks/use-credits";
import { useCanvasMeta } from "@/hooks/use-canvas";
import { useProfile } from "@/hooks/use-profile";
import { LoginDialog } from "@/components/auth/login-dialog";
import { LogIn, LogOut, Palette } from "lucide-react";

export function Header() {
  const [loginOpen, setLoginOpen] = useState(false);
  const isAuthenticated = useAuthStore((s) => s.isAuthenticated);
  const publicKey = useAuthStore((s) => s.publicKey);
  const logout = useAuthStore((s) => s.logout);
  const isHydrated = useAuthStore((s) => s.isHydrated);
  const { data: credits } = useCredits();
  const { data: meta } = useCanvasMeta();
  const { data: profile } = useProfile(publicKey);

  const displayName = profile?.name
    ?? (publicKey ? `${publicKey.slice(0, 8)}...${publicKey.slice(-4)}` : "");

  return (
    <>
      <header className="flex items-center justify-between border-b border-neutral-800 bg-neutral-950 px-4 py-3">
        <div className="flex items-center gap-3">
          <Palette size={22} className="text-blue-400" />
          <h1 className="text-lg font-bold text-white">Pubky Canva</h1>
          {meta && (
            <span className="hidden text-xs text-neutral-500 sm:inline">
              {meta.width}x{meta.height} | {meta.filled}/{meta.total_pixels} filled
              | {meta.overwritten} overwritten
            </span>
          )}
        </div>

        <div className="flex items-center gap-3">
          {isAuthenticated && credits && (
            <div className="flex items-center gap-1.5 rounded-lg bg-neutral-800 px-3 py-1.5">
              <div
                className="h-2.5 w-2.5 rounded-full"
                style={{
                  backgroundColor:
                    credits.credits > 0 ? "#00E436" : "#FF004D",
                }}
              />
              <span className="text-sm font-medium text-white">
                {credits.credits}/{credits.max_credits}
              </span>
              {credits.next_credit_in_seconds != null && credits.credits < credits.max_credits && (
                <span className="text-xs text-neutral-400">
                  +1 in {Math.ceil(credits.next_credit_in_seconds)}s
                </span>
              )}
            </div>
          )}

          {isHydrated && (
            isAuthenticated ? (
              <div className="flex items-center gap-2">
                <span className="text-xs text-neutral-400">{displayName}</span>
                <button
                  onClick={logout}
                  className="rounded-lg p-2 text-neutral-400 hover:bg-neutral-800 hover:text-white"
                  title="Disconnect"
                >
                  <LogOut size={16} />
                </button>
              </div>
            ) : (
              <button
                onClick={() => setLoginOpen(true)}
                className="flex items-center gap-2 rounded-lg bg-blue-600 px-3 py-1.5 text-sm font-medium text-white hover:bg-blue-500"
              >
                <LogIn size={14} />
                Connect
              </button>
            )
          )}
        </div>
      </header>

      <LoginDialog open={loginOpen} onClose={() => setLoginOpen(false)} />
    </>
  );
}
