/** @type {import('next').NextConfig} */
const nextConfig = {
  images: {
    unoptimized: true,
  },
  async redirects() {
    return [
      {
        source: '/',
        destination: '/world-map-demo/world-map.html',
        permanent: false,
      },
    ]
  },
}

export default nextConfig
