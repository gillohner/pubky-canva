import type { Keypair, Session } from "@synonymdev/pubky";

export interface AuthData {
  isAuthenticated: boolean;
  publicKey: string | null;
  keypair: Keypair | null;
  session: Session | null;
}
