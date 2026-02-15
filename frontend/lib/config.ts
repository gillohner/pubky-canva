interface Config {
  indexerUrl: string;
  relay: {
    url: string;
  };
  pubkyApp: {
    profileUrl: string;
  };
}

function buildConfig(): Config {
  return {
    indexerUrl:
      process.env.NEXT_PUBLIC_INDEXER_URL || "http://localhost:3001",
    relay: {
      url:
        process.env.NEXT_PUBLIC_RELAY_URL ||
        "https://httprelay.pubky.app/link/",
    },
    pubkyApp: {
      profileUrl:
        process.env.NEXT_PUBLIC_PUBKY_APP_URL
          ? `${process.env.NEXT_PUBLIC_PUBKY_APP_URL}/profile`
          : "https://pubky.app/profile",
    },
  };
}

export const config = buildConfig();
