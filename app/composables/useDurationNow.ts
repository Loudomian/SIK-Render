const durationNow = ref(Date.now())
let durationNowTimer: number | null = null
let durationNowConsumers = 0

export function useDurationNow() {
  onMounted(() => {
    durationNowConsumers += 1
    durationNow.value = Date.now()

    if (!durationNowTimer) {
      durationNowTimer = window.setInterval(() => {
        durationNow.value = Date.now()
      }, 1000)
    }
  })

  onUnmounted(() => {
    durationNowConsumers = Math.max(0, durationNowConsumers - 1)
    if (durationNowConsumers === 0 && durationNowTimer) {
      window.clearInterval(durationNowTimer)
      durationNowTimer = null
    }
  })

  return durationNow
}
