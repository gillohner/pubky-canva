import { Pubky, Keypair, Session } from "@synonymdev/pubky";

export class PubkyClient {
  static restoreFromRecoveryFile(
    recoveryFile: Uint8Array,
    passphrase: string
  ): Keypair {
    return Keypair.fromRecoveryFile(recoveryFile, passphrase);
  }

  async signin(keypair: Keypair): Promise<Session> {
    const pubky = new Pubky();
    const signer = pubky.signer(keypair);
    return signer.signin();
  }

  async restoreSession(sessionExport: string): Promise<Session> {
    const pubky = new Pubky();
    return pubky.restoreSession(sessionExport);
  }
}

export const pubkyClient = new PubkyClient();
