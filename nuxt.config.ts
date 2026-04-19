// https://nuxt.com/docs/api/configuration/nuxt-config
export default defineNuxtConfig({
  ssr: false, // Tauri requires SPA mode
  srcDir: 'app/',

  devServer: {
    port: 3000,
  },

  modules: ['@pinia/nuxt'],

  css: ['~/assets/css/main.css'],

  vite: {
    // Prevent Vite from obscuring Rust errors
    clearScreen: false,
    envPrefix: ['VITE_', 'TAURI_'],
    server: {
      strictPort: true,
    },
  },

  typescript: {
    strict: true,
  },

  compatibilityDate: '2025-01-01',
})
