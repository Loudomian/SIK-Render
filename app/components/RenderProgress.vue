<template>
  <div class="render-progress">
    <UProgress
      :model-value="percent"
      :max="100"
      color="info"
      size="sm"
      class="progress-bar"
    />
    <div class="progress-meta">
      <span v-if="isWarmingUp" class="progress-label">Blender 预热中</span>
      <template v-else>
        <span class="progress-label">第 {{ frame }} / {{ totalFrames }} 帧（{{ percent }}%）</span>
        <span class="progress-time">
          单帧 {{ timeElapsed != null && timeElapsed > 0 ? fmtSecs(timeElapsed) : '计算中' }}
        </span>
        <span class="progress-remaining">
          预计 {{ displayRemainingSecs != null && displayRemainingSecs > 0 ? fmtSecs(displayRemainingSecs) : '计算中' }}
        </span>
      </template>
    </div>
  </div>
</template>

<script setup lang="ts">
const props = defineProps<{
  frame: number
  totalFrames: number
  warmingUp?: boolean
  timeElapsed?: number
  remainingSecs?: number | null
}>()

const percent = computed(() =>
  props.totalFrames > 0 ? Math.round((props.frame / props.totalFrames) * 100) : 0,
)

const isWarmingUp = computed(() => props.warmingUp ?? false)

const displayRemainingSecs = computed(() => {
  if (props.remainingSecs != null && props.remainingSecs > 0) {
    return props.remainingSecs
  }

  if (isWarmingUp.value) {
    return null
  }

  if (props.timeElapsed != null && props.timeElapsed > 0 && props.totalFrames > props.frame) {
    return props.timeElapsed * (props.totalFrames - props.frame)
  }

  return null
})

function fmtSecs(s: number): string {
  const total = Math.round(s)
  if (total < 60) return `${total}s`
  const m = Math.floor(total / 60)
  const sec = total % 60
  if (m < 60) return `${m}m ${sec}s`
  return `${Math.floor(m / 60)}h ${m % 60}m`
}
</script>
