import { readFileSync } from 'node:fs'

// https://nuxt.com/docs/api/configuration/nuxt-config
const buildDir = process.env.NUXT_BUILD_DIR || '.nuxt-dev'
const viteCacheDir = process.env.VITE_CACHE_DIR || `.vite-cache/${buildDir.replace(/[\\/]/g, '-')}`
const packageVersion = JSON.parse(readFileSync(new URL('./package.json', import.meta.url), 'utf8')).version
const themeBootstrapScript = `(function(){try{var storedTheme=window.localStorage.getItem('sik-render-theme')||'dark';var resolvedTheme=storedTheme==='system'?(window.matchMedia('(prefers-color-scheme: dark)').matches?'dark':'light'):storedTheme;var root=document.documentElement;root.classList.toggle('dark',resolvedTheme==='dark');root.classList.toggle('light',resolvedTheme!=='dark');root.style.colorScheme=resolvedTheme==='dark'?'dark':'light';}catch(_){var root=document.documentElement;root.classList.add('dark');root.classList.remove('light');root.style.colorScheme='dark';}})();`

export default defineNuxtConfig({
  ssr: false, // Tauri requires SPA mode
  srcDir: 'app/',
  buildDir,
  app: {
    head: {
      script: [
        {
          key: 'theme-bootstrap',
          innerHTML: themeBootstrapScript,
          tagPosition: 'head',
        },
      ],
    },
  },
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
    vue: {
      template: {
        compilerOptions: {
          isCustomElement: tag => tag.startsWith('media-'),
        },
      },
    },
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

  runtimeConfig: {
    public: {
      appVersion: packageVersion,
    },
  },

  compatibilityDate: '2025-01-01',
})
