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
import type { CommunitySkinTranslations, MediaPlayerElement } from 'vidstack'

defineProps<{
  src: string
  poster?: string | null
  title: string
}>()

const playerEl = ref<MediaPlayerElement | null>(null)
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

function bindPlayerEvents(player: MediaPlayerElement, onCleanup: (cleanupFn: () => void) => void) {
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

const translations: CommunitySkinTranslations = {
  Audio: '音频',
  Auto: '自动',
  Captions: '字幕',
  Chapters: '章节',
  Default: '默认',
  Mute: '静音',
  Normal: '正常',
  Off: '关闭',
  Pause: '暂停',
  Play: '播放',
  Speed: '倍速',
  Quality: '画质',
  Settings: '设置',
  Unmute: '取消静音',
  'Seek Forward': '快进',
  'Seek Backward': '后退',
  'Closed-Captions On': '开启字幕',
  'Closed-Captions Off': '关闭字幕',
  'Enter Fullscreen': '进入全屏',
  'Exit Fullscreen': '退出全屏',
  'Enter PiP': '开启画中画',
  'Exit PiP': '退出画中画',
}
</script>
