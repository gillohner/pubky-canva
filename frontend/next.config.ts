import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  // Prevent server-side bundling of WASM packages
  serverExternalPackages: ["@synonymdev/pubky"],

  // Webpack config for production builds (Turbopack only used in dev)
  webpack: (config, { isServer }) => {
    config.experiments = {
      ...config.experiments,
      asyncWebAssembly: true,
      layers: true,
    };

    if (isServer) {
      config.externals = config.externals || [];
      config.externals.push({
        "@synonymdev/pubky": "commonjs @synonymdev/pubky",
      });
    }

    return config;
  },
};

export default nextConfig;
