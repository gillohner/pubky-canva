"use client";

import type { AuthData } from "@/types/auth";
import type { Keypair, Session } from "@synonymdev/pubky";
import { create } from "zustand";
import { persist, createJSONStorage } from "zustand/middleware";
import { ingestUser } from "@/lib/api/canvas";

const STORAGE_KEY = "pubky-canva-auth";

interface AuthStore extends AuthData {
  sessionExport: string | null;
  isHydrated: boolean;
  isRestoringSession: boolean;
  signin: (publicKey: string, keypair: Keypair, session: Session) => void;
  signinWithSession: (publicKey: string, session: Session) => void;
  restoreSessionFromExport: () => Promise<void>;
  logout: () => void;
  setIsHydrated: (v: boolean) => void;
}

const safeSessionExport = (session: Session | null): string | null => {
  if (!session) return null;
  try {
    return typeof session.export === "function" ? session.export() : null;
  } catch {
    return null;
  }
};

export const useAuthStore = create<AuthStore>()(
  persist(
    (set, get) => ({
      isAuthenticated: false,
      publicKey: null,
      keypair: null,
      session: null,
      sessionExport: null,
      isHydrated: false,
      isRestoringSession: false,

      signin: (publicKey, keypair, session) => {
        set({
          isAuthenticated: true,
          publicKey,
          keypair,
          session,
          sessionExport: safeSessionExport(session),
        });
        ingestUser(publicKey).catch(console.error);
      },

      signinWithSession: (publicKey, session) => {
        set({
          isAuthenticated: true,
          publicKey,
          keypair: null,
          session,
          sessionExport: safeSessionExport(session),
        });
        ingestUser(publicKey).catch(console.error);
      },

      restoreSessionFromExport: async () => {
        const { sessionExport, session, publicKey } = get();
        if (!sessionExport || session) return;

        set({ isRestoringSession: true });

        try {
          const { pubkyClient } = await import("@/lib/pubky/client");
          const restoredSession = await Promise.race([
            pubkyClient.restoreSession(sessionExport),
            new Promise<Session>((_, reject) =>
              setTimeout(() => reject(new Error("timeout")), 8000)
            ),
          ]);

          set({
            isAuthenticated: true,
            publicKey,
            keypair: null,
            session: restoredSession,
            sessionExport: safeSessionExport(restoredSession),
            isRestoringSession: false,
          });

          if (publicKey) {
            ingestUser(publicKey).catch(console.error);
          }
        } catch {
          set({
            isAuthenticated: false,
            publicKey: null,
            keypair: null,
            session: null,
            sessionExport: null,
            isRestoringSession: false,
          });
        }
      },

      logout: () => {
        set({
          isAuthenticated: false,
          publicKey: null,
          keypair: null,
          session: null,
          sessionExport: null,
          isHydrated: true,
        });
      },

      setIsHydrated: (v) => set({ isHydrated: v }),
    }),
    {
      name: STORAGE_KEY,
      version: 1,
      storage: createJSONStorage(() => localStorage),
      partialize: (state) => ({
        publicKey: state.publicKey,
        sessionExport: state.sessionExport,
      }),
      onRehydrateStorage: () => (state) => {
        if (state) {
          state.setIsHydrated(true);
          if (state.sessionExport) {
            state.restoreSessionFromExport();
          }
        }
      },
    }
  )
);
