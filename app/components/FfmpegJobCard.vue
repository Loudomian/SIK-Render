<template>
  <UCard
    class="job-card job-card-openable"
    :class="`status-${job.status}`"
    variant="subtle"
    :ui="{ body: 'job-card-body' }"
  >
    <div class="job-card-layout ffmpeg-card-layout">
      <div class="job-card-info">
        <div class="job-card-heading">
          <div class="job-title-stack">
            <div class="job-head-badges">
              <UBadge :label="statusLabel" :color="statusColor" variant="subtle" />
              <UBadge
                v-if="orderBadgeLabel"
                :label="orderBadgeLabel"
                color="neutral"
                variant="subtle"
              />
              <UBadge
                :label="job.sourceType === 'blender_job' ? t('ffmpegCard.sourceBlenderJob') : t('ffmpegCard.sourceFolder')"
                color="neutral"
                variant="subtle"
              />
            </div>
            <span class="job-name">
              <span class="job-number">#{{ job.jobNumber }}</span> {{ job.name }}
            </span>
            <p class="job-meta-path" :title="job.inputPath">{{ job.inputPath }}</p>
          </div>
        </div>

        <div class="job-footer">
          <div class="job-meta">
            <span class="job-meta-item">
              <span class="job-meta-label">{{ t('ffmpegCard.frameSegment') }}</span>
              <strong>{{ job.frameStart }}–{{ job.frameEnd }}</strong>
            </span>
            <span class="job-meta-divider" aria-hidden="true" />
            <span class="job-meta-item">
              <span class="job-meta-label">FPS</span>
              <strong>{{ job.fps.toFixed(3) }}</strong>
            </span>
            <span class="job-meta-divider" aria-hidden="true" />
            <span class="job-meta-item">
              <span class="job-meta-label">CRF / Preset</span>
              <strong>{{ job.crf }} / {{ job.preset }}</strong>
            </span>
            <template v-if="job.outputSizeBytes">
              <span class="job-meta-divider" aria-hidden="true" />
              <span class="job-meta-item">
                <span class="job-meta-label">{{ t('ffmpegCard.fileSize') }}</span>
                <strong>{{ formatBytes(job.outputSizeBytes) }}</strong>
              </span>
            </template>
          </div>

          <div class="job-actions" data-no-drag @dblclick.stop>
            <UButton
              v-if="job.status === 'running'"
              icon="i-lucide-square"
              :label="t('common.cancel')"
              color="warning"
              variant="outline"
              size="sm"
              @click="$emit('cancel')"
            />
            <UButton
              v-if="job.status !== 'running'"
              icon="i-lucide-trash-2"
              :label="t('common.delete')"
              color="error"
              variant="outline"
              size="sm"
              @click="$emit('remove')"
            />
            <UButton
              v-if="job.outputPath"
              icon="i-lucide-folder-open"
              :label="t('common.outputDirectory')"
              color="neutral"
              variant="outline"
              size="sm"
              @click="openOutputDirectory"
            />
            <UButton
              :to="`/transcode/${job.id}`"
              icon="i-lucide-external-link"
              :label="t('common.details')"
              color="neutral"
              variant="outline"
              size="sm"
            />
          </div>
        </div>
      </div>
    </div>

    <div v-if="job.status === 'running'" class="job-card-progress ffmpeg-progress">
      <UProgress
        :value="job.progressFrame ?? 0"
        :max="job.totalFrames ?? totalFrames"
        size="sm"
      />
      <div class="progress-meta">
        <span>{{ job.progressFrame ?? 0 }} / {{ job.totalFrames ?? totalFrames }} {{ t('ffmpegCard.frames') }}</span>
      </div>
    </div>
  </UCard>
</template>

<script setup lang="ts">
import type { FfmpegJob } from '~/types'
import { FFMPEG_STATUS_COLOR, useFfmpegStatusLabel } from '~/composables/useFfmpegStatus'
import { FFMPEG_QUEUE_ORDER_HIDDEN_STATUSES, resolveQueueOrder, useQueueOrderLabel } from '~/composables/useQueueOrder'
import { resolveOutputDirectory } from '~/utils/output-path'

const props = defineProps<{ job: FfmpegJob }>()

defineEmits<{
  cancel: []
  remove: []
}>()

const { openPath } = useTauri()
const transcodeStore = useTranscodeStore()
const { t } = useI18n()

const translatedStatusLabel = useFfmpegStatusLabel()
const queueOrderLabel = useQueueOrderLabel()
const statusLabel = computed(() => translatedStatusLabel(props.job.status))
const statusColor = computed(() => FFMPEG_STATUS_COLOR[props.job.status])
const totalFrames = computed(() => props.job.frameEnd - props.job.frameStart + 1)
const queueOrder = computed(() => {
  return resolveQueueOrder(transcodeStore.ffmpegJobs, props.job, FFMPEG_QUEUE_ORDER_HIDDEN_STATUSES)
})
const orderBadgeLabel = computed(() => queueOrderLabel(queueOrder.value))

function openOutputDirectory() {
  openPath(resolveOutputDirectory(props.job.outputPath))
}

function formatBytes(value: number | null) {
  if (!value || value <= 0) return '—'
  if (value < 1024 * 1024) return `${(value / 1024).toFixed(1)} KB`
  if (value < 1024 * 1024 * 1024) return `${(value / (1024 * 1024)).toFixed(1)} MB`
  return `${(value / (1024 * 1024 * 1024)).toFixed(1)} GB`
}
</script>
