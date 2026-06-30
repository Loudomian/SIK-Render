import { nextTick, onMounted, onUnmounted, ref } from 'vue'

export function useCardInfoHeight() {
  const cardInfoEl = ref<HTMLElement | null>(null)
  const cardInfoHeight = ref<number | null>(null)
  let resizeObserver: ResizeObserver | null = null

  function syncHeight() {
    const el = cardInfoEl.value
    if (!el) return
    cardInfoHeight.value = Math.round(el.getBoundingClientRect().height)
  }

  async function syncHeightAfterTick() {
    await nextTick()
    syncHeight()
  }

  onMounted(() => {
    if (!cardInfoEl.value) return
    syncHeight()
    resizeObserver = new ResizeObserver((entries) => {
      const entry = entries[0]
      if (!entry) return
      cardInfoHeight.value = Math.round(entry.contentRect.height)
    })
    resizeObserver.observe(cardInfoEl.value)
  })

  onUnmounted(() => {
    resizeObserver?.disconnect()
  })

  return { cardInfoEl, cardInfoHeight, syncHeight, syncHeightAfterTick }
}
