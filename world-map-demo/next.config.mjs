/** @type {import('next').NextConfig} */
const nextConfig = {
  async rewrites() {
    return [
      { source: "/", destination: "/world-map.html" },
    ];
  },
};

export default nextConfig;
