import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  turbopack: {},
  async rewrites() {
    const servicesUrl =
      process.env.NEXT_PUBLIC_DIKE_SERVICES_URL ?? "http://localhost:4000";
    return [
      {
        source: "/api/proxy/:path*",
        destination: `${servicesUrl}/:path*`,
      },
    ];
  },
};

export default nextConfig;
