import { defineNuxtPlugin } from '#app'
import { defineCustomElements } from 'vidstack/elements'

export default defineNuxtPlugin(async () => {
  await defineCustomElements()
})
