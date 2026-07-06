<template>
  <div class="preview-player-shell">
    <media-player
      ref="playerEl"
      :key="src"
      :src="src"
      :poster="poster || undefined"
      :title="title"
      aspect-ratio="16/9"
      load="eager"
      preload="metadata"
      playsinline
      key-target="player"
      class="preview-player"
    >
      <media-outlet />
      <media-community-skin :translations="translations" />
    </media-player>
  </div>
</template>

<script setup lang="ts">
import type { CommunitySkinTranslations } from 'vidstack'

type PreviewPlayerElement = HTMLElement & {
  readonly paused: boolean
  pause: () => Promise<void>
}

defineProps<{
  src: string
  poster?: string | null
  title: string
}>()

const playerEl = ref<PreviewPlayerElement | null>(null)
const { t } = useI18n()
let keepPausedAfterSeekUntil = 0
let resetSeekGuardTimer = 0

function guardPausedSeek() {
  keepPausedAfterSeekUntil = performance.now() + 260
  window.clearTimeout(resetSeekGuardTimer)
}

function clearPausedSeekGuard(delay = 0) {
  window.clearTimeout(resetSeekGuardTimer)
  if (delay <= 0) {
    keepPausedAfterSeekUntil = 0
    return
  }

  resetSeekGuardTimer = window.setTimeout(() => {
    keepPausedAfterSeekUntil = 0
  }, delay)
}

function handleSeekRequest() {
  if (playerEl.value?.paused) {
    guardPausedSeek()
  }
}

function handleSeeked() {
  const player = playerEl.value
  if (!player) return
  if (performance.now() >= keepPausedAfterSeekUntil) return

  if (!player.paused) {
    void player.pause().catch(() => {})
  }

  clearPausedSeekGuard(80)
}

function handlePlay() {
  const player = playerEl.value
  if (!player) return
  if (performance.now() >= keepPausedAfterSeekUntil) return

  queueMicrotask(() => {
    if (!playerEl.value || playerEl.value !== player || player.paused) return
    void player.pause().catch(() => {})
    clearPausedSeekGuard(80)
  })
}

function bindPlayerEvents(player: PreviewPlayerElement, onCleanup: (cleanupFn: () => void) => void) {
  const seekRequestHandler = () => handleSeekRequest()
  const seekedHandler = () => handleSeeked()
  const playHandler = () => handlePlay()
  const pauseHandler = () => clearPausedSeekGuard()

  player.addEventListener('media-seek-request', seekRequestHandler)
  player.addEventListener('media-seeking-request', seekRequestHandler)
  player.addEventListener('seeked', seekedHandler)
  player.addEventListener('play', playHandler)
  player.addEventListener('pause', pauseHandler)

  onCleanup(() => {
    player.removeEventListener('media-seek-request', seekRequestHandler)
    player.removeEventListener('media-seeking-request', seekRequestHandler)
    player.removeEventListener('seeked', seekedHandler)
    player.removeEventListener('play', playHandler)
    player.removeEventListener('pause', pauseHandler)
  })
}

watch(
  playerEl,
  (player, _previous, onCleanup) => {
    clearPausedSeekGuard()
    if (!player) return
    bindPlayerEvents(player, onCleanup)
  },
  { immediate: true },
)

onUnmounted(() => {
  window.clearTimeout(resetSeekGuardTimer)
})

const translations = computed<CommunitySkinTranslations>(() => ({
  Audio: t('videoPlayer.audio'),
  Auto: t('videoPlayer.auto'),
  Captions: t('videoPlayer.captions'),
  Chapters: t('videoPlayer.chapters'),
  Default: t('videoPlayer.default'),
  Mute: t('videoPlayer.mute'),
  Normal: t('videoPlayer.normal'),
  Off: t('videoPlayer.off'),
  Pause: t('videoPlayer.pause'),
  Play: t('videoPlayer.play'),
  Speed: t('videoPlayer.speed'),
  Quality: t('videoPlayer.quality'),
  Settings: t('videoPlayer.settings'),
  Unmute: t('videoPlayer.unmute'),
  'Seek Forward': t('videoPlayer.seekForward'),
  'Seek Backward': t('videoPlayer.seekBackward'),
  'Closed-Captions On': t('videoPlayer.captionsOn'),
  'Closed-Captions Off': t('videoPlayer.captionsOff'),
  'Enter Fullscreen': t('videoPlayer.enterFullscreen'),
  'Exit Fullscreen': t('videoPlayer.exitFullscreen'),
  'Enter PiP': t('videoPlayer.enterPip'),
  'Exit PiP': t('videoPlayer.exitPip'),
}))
</script>
