export default defineAppConfig({
  ui: {
    colors: {
      primary: 'emerald',
      secondary: 'zinc',
      success: 'emerald',
      info: 'sky',
      warning: 'amber',
      error: 'rose',
      neutral: 'zinc',
    },
    card: {
      slots: {
        root: 'surface-card overflow-hidden',
        header: 'surface-card-header p-4 sm:px-6',
        body: 'surface-card-body-slot p-4 sm:p-6',
        footer: 'surface-card-footer p-4 sm:px-6',
      },
      variants: {
        variant: {
          solid: {
            root: 'surface-card-solid',
          },
          outline: {
            root: 'surface-card-outline',
          },
          soft: {
            root: 'surface-card-soft',
          },
          subtle: {
            root: 'surface-card-subtle',
          },
        },
      },
      defaultVariants: {
        variant: 'subtle',
      },
    },
  },
})
