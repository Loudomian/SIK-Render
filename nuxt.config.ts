// https://nuxt.com/docs/api/configuration/nuxt-config
const buildDir = process.env.NUXT_BUILD_DIR || '.nuxt-dev'
const viteCacheDir = process.env.VITE_CACHE_DIR || `.vite-cache/${buildDir.replace(/[\\/]/g, '-')}`

export default defineNuxtConfig({
  ssr: false, // Tauri requires SPA mode
  srcDir: 'app/',
  buildDir,
  devtools: {
    enabled: false,
  },
  experimental: {
    appManifest: false,
  },

  devServer: {
    port: 3000,
  },

  modules: ['@pinia/nuxt', '@nuxt/icon', '@nuxt/ui'],

  icon: {
    provider: 'none',
    clientBundle: {
      scan: true,
    },
  },

  ui: {
    fonts: false,
  },

  css: [
    '~/assets/css/ui.css',
    '~/assets/css/colors.css',
    '~/assets/css/framework.css',
  ],

  vite: {
    cacheDir: viteCacheDir,
    optimizeDeps: {
      include: [
        '@tauri-apps/api/core',
        '@tauri-apps/api/event',
        '@tauri-apps/api/window',
        '@tauri-apps/plugin-dialog',
      ],
    },
    // Prevent Vite from obscuring Rust errors
    clearScreen: false,
    envPrefix: ['VITE_', 'TAURI_'],
    server: {
      strictPort: true,
      watch: {
        ignored: [
          '**/logs/**',
          '**/src-tauri/target/**',
          '**/.output/**',
          '**/.nuxt/**',
          '**/.nuxt-dev*/**',
          '**/.nuxt-build/**',
        ],
      },
    },
  },

  typescript: {
    strict: true,
  },

  compatibilityDate: '2025-01-01',
})
