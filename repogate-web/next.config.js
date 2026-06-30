/** @type {import('next').NextConfig} */
// Static export: the Rust server (repogate-server --static-dir out/) serves the
// build and the API from the same origin, so no dev rewrites are needed.
// For local dev against a separate API, set NEXT_PUBLIC_API_BASE.
const nextConfig = {
  output: 'export',
  trailingSlash: true,
  images: { unoptimized: true },
};

module.exports = nextConfig;
