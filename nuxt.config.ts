// https://nuxt.com/docs/api/configuration/nuxt-config
export default defineNuxtConfig({
  ssr: false, // Tauri requires SPA mode
  srcDir: 'app/',
  buildDir: '.nuxt-build',
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
