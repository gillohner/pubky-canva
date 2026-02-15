"use client";

import { useState, useRef, useEffect, useCallback } from "react";
import { useAuthStore } from "@/stores/auth-store";
import { PubkyClient } from "@/lib/pubky/client";
import { config } from "@/lib/config";
import QRCode from "qrcode";
import * as pubky from "@synonymdev/pubky";
import { X, QrCode, FileKey, Copy, Check, ExternalLink } from "lucide-react";

interface LoginDialogProps {
  open: boolean;
  onClose: () => void;
}

type Tab = "qr" | "recovery";

export function LoginDialog({ open, onClose }: LoginDialogProps) {
  const signin = useAuthStore((s) => s.signin);
  const signinWithSession = useAuthStore((s) => s.signinWithSession);
  const [tab, setTab] = useState<Tab>("qr");
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  // Recovery file state
  const [passphrase, setPassphrase] = useState("");
  const fileRef = useRef<HTMLInputElement>(null);

  // QR code state
  const [authUrl, setAuthUrl] = useState("");
  const [copied, setCopied] = useState(false);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const sdkRef = useRef<pubky.Pubky | null>(null);

  // Reset state on open/close
  useEffect(() => {
    if (open) {
      setError(null);
      setAuthUrl("");
      setCopied(false);
    }
  }, [open]);

  // Initialize SDK for QR auth
  useEffect(() => {
    if (!open) return;
    sdkRef.current = new pubky.Pubky();
  }, [open]);

  // Render QR code when authUrl changes
  useEffect(() => {
    if (!canvasRef.current || !authUrl) return;
    QRCode.toCanvas(canvasRef.current, authUrl, {
      margin: 2,
      width: 192,
      color: { light: "#ffffff", dark: "#000000" },
    }).catch(console.error);
  }, [authUrl]);

  // Start QR auth flow
  const startQrFlow = useCallback(async () => {
    if (!sdkRef.current) return;
    setError(null);
    setAuthUrl("");

    try {
      const caps = "/pub/pubky-canva/:rw" as pubky.Capabilities;
      const flowKind = pubky.AuthFlowKind.signin();
      const flow = sdkRef.current.startAuthFlow(caps, flowKind, config.relay.url);
      setAuthUrl(flow.authorizationUrl);

      const session = await flow.awaitApproval();
      const publicKey = session.info.publicKey.z32();
      signinWithSession(publicKey, session);
      onClose();
    } catch (e) {
      setError(e instanceof Error ? e.message : "QR auth failed");
    }
  }, [signinWithSession, onClose]);

  // Auto-start QR flow when tab is qr
  useEffect(() => {
    if (open && tab === "qr" && !authUrl && sdkRef.current) {
      const timer = setTimeout(startQrFlow, 100);
      return () => clearTimeout(timer);
    }
  }, [open, tab, authUrl, startQrFlow]);

  const handleCopyUrl = async () => {
    if (!authUrl) return;
    await navigator.clipboard.writeText(authUrl);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  const handleOpenInRing = () => {
    if (!authUrl) return;
    const opened = window.open(authUrl, "_blank");
    if (!opened) window.location.href = authUrl;
  };

  const handleRecoveryFile = async () => {
    const file = fileRef.current?.files?.[0];
    if (!file) {
      setError("Please select a recovery file");
      return;
    }
    setLoading(true);
    setError(null);
    try {
      const bytes = new Uint8Array(await file.arrayBuffer());
      const keypair = PubkyClient.restoreFromRecoveryFile(bytes, passphrase);
      const sdk = new pubky.Pubky();
      const signer = sdk.signer(keypair);
      const session = await signer.signin();
      const pk = keypair.publicKey.z32();
      signin(pk, keypair, session);
      onClose();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Login failed");
    } finally {
      setLoading(false);
    }
  };

  if (!open) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm">
      <div className="relative w-full max-w-sm rounded-xl border border-neutral-700 bg-neutral-900 p-6 shadow-2xl">
        <button
          onClick={onClose}
          className="absolute right-3 top-3 text-neutral-400 hover:text-white"
        >
          <X size={18} />
        </button>

        <h2 className="mb-4 text-lg font-semibold text-white">Connect</h2>

        {/* Tabs */}
        <div className="mb-4 flex gap-1 rounded-lg bg-neutral-800 p-1">
          <TabButton
            active={tab === "qr"}
            onClick={() => { setTab("qr"); setError(null); }}
            icon={<QrCode size={14} />}
            label="Pubky Ring"
          />
          <TabButton
            active={tab === "recovery"}
            onClick={() => { setTab("recovery"); setError(null); }}
            icon={<FileKey size={14} />}
            label="Recovery File"
          />
        </div>

        {/* QR Code Tab */}
        {tab === "qr" && (
          <div className="flex flex-col items-center space-y-3">
            <p className="text-center text-xs text-neutral-400">
              Scan with Pubky Ring to connect
            </p>
            <div className="rounded-xl bg-white p-3">
              <canvas ref={canvasRef} className="h-48 w-48" />
            </div>
            {authUrl && (
              <div className="flex w-full gap-2">
                <button
                  onClick={handleCopyUrl}
                  className="flex flex-1 items-center justify-center gap-1.5 rounded-lg border border-neutral-700 bg-neutral-800 px-3 py-2 text-xs text-neutral-300 hover:bg-neutral-700"
                >
                  {copied ? <Check size={12} /> : <Copy size={12} />}
                  {copied ? "Copied" : "Copy Link"}
                </button>
                <button
                  onClick={handleOpenInRing}
                  className="flex flex-1 items-center justify-center gap-1.5 rounded-lg bg-blue-600 px-3 py-2 text-xs text-white hover:bg-blue-500"
                >
                  <ExternalLink size={12} />
                  Open in Ring
                </button>
              </div>
            )}
          </div>
        )}

        {/* Recovery File Tab */}
        {tab === "recovery" && (
          <div className="space-y-4">
            <div>
              <label className="mb-1 block text-sm text-neutral-400">
                Recovery File (.pkarr)
              </label>
              <input
                ref={fileRef}
                type="file"
                accept=".pkarr"
                className="w-full rounded-lg border border-neutral-700 bg-neutral-800 px-3 py-2 text-sm text-white file:mr-3 file:rounded file:border-0 file:bg-neutral-700 file:px-2 file:py-1 file:text-sm file:text-white"
              />
            </div>
            <div>
              <label className="mb-1 block text-sm text-neutral-400">
                Passphrase
              </label>
              <input
                type="password"
                value={passphrase}
                onChange={(e) => setPassphrase(e.target.value)}
                placeholder="Enter passphrase"
                className="w-full rounded-lg border border-neutral-700 bg-neutral-800 px-3 py-2 text-sm text-white placeholder:text-neutral-500"
              />
            </div>
            <button
              onClick={handleRecoveryFile}
              disabled={loading}
              className="w-full rounded-lg bg-blue-600 px-4 py-2 text-sm font-medium text-white hover:bg-blue-500 disabled:opacity-50"
            >
              {loading ? "Connecting..." : "Sign In"}
            </button>
          </div>
        )}

        {error && <p className="mt-3 text-sm text-red-400">{error}</p>}
      </div>
    </div>
  );
}

function TabButton({
  active,
  onClick,
  icon,
  label,
}: {
  active: boolean;
  onClick: () => void;
  icon?: React.ReactNode;
  label: string;
}) {
  return (
    <button
      onClick={onClick}
      className={`flex flex-1 items-center justify-center gap-1.5 rounded-md px-2 py-1.5 text-xs font-medium transition-colors ${
        active
          ? "bg-neutral-700 text-white"
          : "text-neutral-400 hover:text-neutral-200"
      }`}
    >
      {icon}
      {label}
    </button>
  );
}
