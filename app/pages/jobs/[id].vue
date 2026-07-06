<template>
  <div v-if="job" class="detail-page">
    <section class="page-hero detail-hero">
      <div class="page-hero-copy detail-title">
        <UContextMenu
          :items="buildJobContextMenuItems(job)"
          :ui="{ content: 'detail-context-menu-content' }"
        >
          <div class="detail-context-menu-target" data-context-menu>
            <div class="queue-heading-row detail-heading-row">
              <div class="queue-heading-copy detail-heading-copy">
                <div class="queue-heading-title detail-heading-title">
                  <UBreadcrumb
                    as="h1"
                    :items="detailBreadcrumbItems"
                    :ui="{
                      root: 'detail-breadcrumb',
                      list: 'detail-breadcrumb-list',
                      item: 'detail-breadcrumb-item',
                      link: 'detail-breadcrumb-link',
                      linkLabel: 'detail-breadcrumb-label',
                      separator: 'detail-breadcrumb-separator-wrap',
                      separatorIcon: 'detail-breadcrumb-separator',
                    }"
                  >
                    <template #separator>
                      <span class="detail-breadcrumb-separator" aria-hidden="true">&gt;</span>
                    </template>
                    <template #item-label="{ item, active }">
                      <span :class="active ? 'detail-breadcrumb-current' : 'detail-breadcrumb-ancestor'">
                        {{ item.label }}
                      </span>
                    </template>
                  </UBreadcrumb>
                  <div class="detail-title-badges">
                    <UBadge
                      :label="statusLabel(job.status)"
                      :color="statusBadgeColor"
                      variant="subtle"
                    />
                    <UBadge v-if="orderBadgeLabel" :label="orderBadgeLabel" color="neutral" variant="subtle" />
                    <UBadge
                      v-if="job.crashCount > 0"
                      :label="t('jobCard.crashCount', { count: job.crashCount })"
                      color="warning"
                      variant="subtle"
                    />
                  </div>
                </div>
                <button
                  type="button"
                  class="page-note detail-note detail-note-edit"
                  :class="{ 'detail-note-empty': !job.note }"
                  :title="t('jobCard.actions.editMetadataTooltip')"
                  @click.stop="openMetadataDialog"
                  @dblclick.stop
                >
                  {{ job.note || t('jobCard.emptyNote') }}
                  <UIcon name="i-lucide-notebook-pen" class="detail-note-icon" />
                </button>
              </div>
              <div class="detail-header-actions">
                <UFieldGroup v-if="transcodeSupported" size="md" class="detail-action-fieldgroup">
                  <UButton
                    :icon="transcodePrimaryAction.icon"
                    :label="transcodePrimaryAction.label"
                    :color="transcodePrimaryAction.color"
                    variant="subtle"
                    size="md"
                    :loading="transcodePrimaryAction.loading"
                    :disabled="transcodePrimaryAction.disabled"
                    :ui="{ leadingIcon: transcodePrimaryAction.spin ? 'spin' : undefined }"
                    @click="handlePrimaryTranscodeAction"
                  />
                  <UDropdownMenu
                    :items="transcodeActionItems"
                    arrow
                    :content="{ side: 'bottom', align: 'end', sideOffset: 8 }"
                  >
                    <UButton
                      icon="i-lucide-chevron-down"
                      color="neutral"
                      variant="outline"
                      size="md"
                      square
                    />
                    <template #auto-transcode-trailing>
                      <USwitch
                        :model-value="autoTranscodeEnabled"
                        color="neutral"
                        :disabled="updatingAutoTranscode || !transcodeSupported"
                        @pointerdown.stop
                        @click.stop
                        @update:model-value="handleAutoTranscodeSwitchUpdate"
                      />
                    </template>
                  </UDropdownMenu>
                </UFieldGroup>
                <UButton
                  v-if="job.status === 'failed' || job.status === 'cancelled' || job.status === 'interrupted' || job.status === 'done'"
                  icon="i-lucide-rotate-ccw"
                  :label="job.status === 'cancelled' || job.status === 'interrupted' ? t('jobDetails.actions.continue') : t('jobDetails.actions.rerender')"
                  :color="job.status === 'cancelled' || job.status === 'interrupted' ? 'warning' : 'neutral'"
                  variant="outline"
                  size="md"
                  @click="handleRetry(job)"
                />
                <UButton
                  v-if="job.status === 'running' || job.status === 'pending'"
                  icon="i-lucide-x"
                  :label="t('common.cancel')"
                  color="warning"
                  variant="outline"
                  size="md"
                  @click="jobsStore.stopJob(job.id)"
                />
                <UButton
                  v-if="job.status === 'done' || job.status === 'failed' || job.status === 'cancelled' || job.status === 'interrupted'"
                  icon="i-lucide-trash-2"
                  :label="t('common.delete')"
                  color="error"
                  variant="outline"
                  size="md"
                  @click="showDeleteConfirm = true"
                />
              </div>
            </div>
          </div>
        </UContextMenu>
      </div>
    </section>

    <UModal
      v-model:open="showDeleteConfirm"
      :close="false"
      :title="t('renderQueue.delete.title')"
      :ui="{ content: 'job-modal-content' }"
    >
      <template #body>
        <div class="modal-stack">
          <p class="modal-copy">
            {{ t('renderQueue.delete.message', { name: job.name }) }}
          </p>
          <div class="modal-actions">
            <UButton icon="i-lucide-x" :label="t('common.cancel')" color="warning" variant="outline" @click="showDeleteConfirm = false" />
            <UButton icon="i-lucide-trash-2" :label="t('common.delete')" color="error" variant="outline" @click="removeAndBack" />
          </div>
        </div>
      </template>
    </UModal>

    <JobMetadataModal
      v-model:open="metadataDialogOpen"
      :job="metadataJob"
    />

    <TranscodeSubmitModal
      v-if="job && transcodeSupported"
      :open="transcodeModalOpen"
      :initial-config="effectiveTranscodeConfig"
      :blender-job-id="job.id"
      :blender-job-blend-file="job.blendFile"
      :blender-job-name="job.name"
      :blender-job-fps="job.fps ?? null"
      :blender-job-frame-start="getEffectiveTranscodeFrameRange(job).frameStart"
      :blender-job-frame-end="getEffectiveTranscodeFrameRange(job).frameEnd"
      :blender-job-original-frame-start="job.originalFrameStart"
      :blender-job-original-frame-end="job.originalFrameEnd"
      :blender-job-output-path="job.outputPath"
      @submit="handleTranscodeSubmit"
      @close="transcodeModalOpen = false"
      @update:open="transcodeModalOpen = $event"
    />

    <TranscodeSubmitModal
      v-if="job && transcodeSupported"
      :open="transcodeSettingsModalOpen"
      mode="settings"
      :initial-config="effectiveTranscodeConfig"
      :base-config="baseTranscodeConfig"
      :blender-job-id="job.id"
      :blender-job-blend-file="job.blendFile"
      :blender-job-name="job.name"
      :blender-job-fps="job.fps ?? null"
      :blender-job-frame-start="getEffectiveTranscodeFrameRange(job).frameStart"
      :blender-job-frame-end="getEffectiveTranscodeFrameRange(job).frameEnd"
      :blender-job-original-frame-start="job.originalFrameStart"
      :blender-job-original-frame-end="job.originalFrameEnd"
      :blender-job-output-path="job.outputPath"
      @save-settings="handleTranscodeSettingsSave"
      @close="transcodeSettingsModalOpen = false"
      @update:open="transcodeSettingsModalOpen = $event"
    />

    <UModal
      :open="showRetryConfirm"
      :close="false"
      :title="t('renderQueue.retry.title')"
      :ui="{ content: 'job-modal-content retry-modal-content' }"
      @update:open="v => { if (!v) closeRetryConfirm() }"
    >
      <template #body>
        <div class="modal-stack">
          <div class="transcode-submit-stack retry-modal-stack">
            <div v-if="retryIsQuickMp4" class="choice-grid retry-choice-grid">
              <section class="surface-panel transcode-submit-section retry-option-section retry-option-section-wide">
                <div class="transcode-submit-head">
                  <div>
                    <p class="choice-card-mode">{{ t('jobCard.quickMp4') }}</p>
                    <h3 class="choice-card-title">{{ t('renderQueue.retry.quickMp4Title') }}</h3>
                  </div>
                </div>
                <p class="choice-card-desc">
                  {{ t('renderQueue.retry.quickMp4Description') }}
                </p>
                <div class="choice-card-toggle-group">
                  <UButton
                    icon="i-lucide-refresh-ccw"
                    :label="t('renderQueue.retry.fullRestart')"
                    color="neutral"
                    variant="outline"
                    size="sm"
                    class="choice-card-toggle-button"
                    :loading="retrySubmittingMode === 'restart'"
                    :disabled="retrySubmittingMode !== null"
                    @click="confirmRetryFromStart"
                  />
                </div>
                <div class="choice-card-note-stack">
                  <p class="choice-card-inline-note">{{ retryFullRangeSummary }}</p>
                </div>
              </section>
            </div>
            <div v-else class="choice-grid retry-choice-grid">
              <section class="surface-panel transcode-submit-section retry-option-section retry-option-section-wide">
                <div class="transcode-submit-head">
                  <div>
                    <p class="choice-card-mode">{{ t('renderQueue.retry.fullSegmentMode') }}</p>
                    <h3 class="choice-card-title">{{ t('renderQueue.retry.fullSegmentTitle') }}</h3>
                  </div>
                </div>
                <p class="choice-card-desc">
                  {{ t('renderQueue.retry.fullSegmentDescription', { range: retryFullRangeLabel }) }}
                </p>
                <div class="choice-card-toggle-group">
                  <UButton
                    icon="i-lucide-chevrons-right"
                    :label="t('renderQueue.retry.fullContinue')"
                    color="neutral"
                    variant="outline"
                    size="sm"
                    class="choice-card-toggle-button"
                    :loading="retrySubmittingMode === 'continue'"
                    :disabled="retrySubmittingMode !== null || job?.status === 'done'"
                    @mouseenter="retryFullRangePreviewMode = 'continue'"
                    @mouseleave="clearRetryPreviewOnLeave($event, 'full')"
                    @focus="retryFullRangePreviewMode = 'continue'"
                    @blur="clearRetryPreviewOnLeave($event, 'full')"
                    @click="confirmRetryContinue"
                  />
                  <UButton
                    icon="i-lucide-refresh-ccw"
                    :label="t('renderQueue.retry.fullRestart')"
                    color="neutral"
                    variant="outline"
                    size="sm"
                    class="choice-card-toggle-button"
                    :loading="retrySubmittingMode === 'restart'"
                    :disabled="retrySubmittingMode !== null"
                    @mouseenter="retryFullRangePreviewMode = 'restart'"
                    @mouseleave="clearRetryPreviewOnLeave($event, 'full')"
                    @focus="retryFullRangePreviewMode = 'restart'"
                    @blur="clearRetryPreviewOnLeave($event, 'full')"
                    @click="confirmRetryFromStart"
                  />
                </div>
                <div class="choice-card-note-stack">
                  <p class="choice-card-inline-note">{{ retryFullRangeSummary }}</p>
                </div>
              </section>

              <section class="surface-panel transcode-submit-section retry-option-section retry-option-section-wide">
                <div class="transcode-submit-head">
                  <div>
                    <p class="choice-card-mode">{{ t('renderQueue.retry.customRangeMode') }}</p>
                    <h3 class="choice-card-title">{{ t('renderQueue.retry.customRangeTitle') }}</h3>
                  </div>
                </div>
                <p class="choice-card-desc">
                  {{ t('renderQueue.retry.customRangeDescription') }}
                </p>
                <div class="choice-card-fields">
                  <UFormField :label="t('renderQueue.retry.startFrame')">
                    <UInputNumber v-model="retryCustomStart" :min="1" />
                  </UFormField>
                  <UFormField :label="t('renderQueue.retry.endFrame')">
                    <UInputNumber v-model="retryCustomEnd" :min="1" />
                  </UFormField>
                </div>
                <div class="choice-card-toggle-group">
                  <UButton
                    icon="i-lucide-chevrons-right"
                    :label="t('renderQueue.retry.rangeContinue')"
                    color="neutral"
                    variant="outline"
                    size="sm"
                    class="choice-card-toggle-button"
                    :loading="retrySubmittingMode === 'range-continue'"
                    :disabled="retrySubmittingMode !== null"
                    @mouseenter="retryCustomPreviewMode = 'continue'"
                    @mouseleave="clearRetryPreviewOnLeave($event, 'custom')"
                    @focus="retryCustomPreviewMode = 'continue'"
                    @blur="clearRetryPreviewOnLeave($event, 'custom')"
                    @click="confirmRetryCustomRange(true)"
                  />
                  <UButton
                    icon="i-lucide-refresh-ccw"
                    :label="t('renderQueue.retry.rangeRestart')"
                    color="neutral"
                    variant="outline"
                    size="sm"
                    class="choice-card-toggle-button"
                    :loading="retrySubmittingMode === 'range-restart'"
                    :disabled="retrySubmittingMode !== null"
                    @mouseenter="retryCustomPreviewMode = 'restart'"
                    @mouseleave="clearRetryPreviewOnLeave($event, 'custom')"
                    @focus="retryCustomPreviewMode = 'restart'"
                    @blur="clearRetryPreviewOnLeave($event, 'custom')"
                    @click="confirmRetryCustomRange(false)"
                  />
                </div>
                <div class="choice-card-note-stack">
                  <p class="choice-card-inline-note">{{ retryCustomActionDescription }}</p>
                  <p v-if="retryCustomInspectLoading" class="choice-card-inline-note">{{ t('renderQueue.retry.checkingRange') }}</p>
                  <p v-else-if="retryCustomRangeSummary" class="choice-card-inline-note">{{ retryCustomRangeSummary }}</p>
                </div>
                <UAlert
                  v-if="retryActionError"
                  color="error"
                  variant="subtle"
                  :description="retryActionError"
                />
              </section>
            </div>

            <section v-if="job && !retryIsQuickMp4 && transcodeSupported" class="surface-panel transcode-submit-section retry-transcode-panel">
              <div class="retry-transcode-head">
                <p class="choice-card-mode">{{ t('renderQueue.retry.autoTranscodeMode') }}</p>
                <h3 class="choice-card-title">{{ t('renderQueue.retry.autoTranscodeTitle') }}</h3>
              </div>
              <p class="choice-card-desc">
                {{ t('renderQueue.retry.autoTranscodeDescription') }}
              </p>
              <USwitch
                v-model="retryAutoTranscodeEnabled"
                color="neutral"
                :label="t('renderQueue.retry.autoTranscodeSwitch')"
                class="choice-card-switch"
              />
              <div class="choice-card-toggle-group">
                <UButton
                  icon="i-lucide-scan-line"
                  :label="t('renderQueue.retry.targetSegment', { range: retryCurrentTargetRangeLabel })"
                  :color="retryTranscodeRangeMode === 'current' ? 'primary' : 'neutral'"
                  :variant="retryTranscodeRangeMode === 'current' ? 'solid' : 'outline'"
                  size="sm"
                  class="choice-card-toggle-button"
                  :disabled="retrySubmittingMode !== null || !retryAutoTranscodeEnabled"
                  @click="retryTranscodeRangeMode = 'current'"
                />
                <UButton
                  icon="i-lucide-film"
                  :label="`${retrySavedTranscodeRangeTitle} ${retryOriginalTranscodeRangeLabel}`"
                  :color="retryTranscodeRangeMode === 'original' ? 'primary' : 'neutral'"
                  :variant="retryTranscodeRangeMode === 'original' ? 'solid' : 'outline'"
                  size="sm"
                  class="choice-card-toggle-button"
                  :disabled="retrySubmittingMode !== null || !retryAutoTranscodeEnabled"
                  @click="retryTranscodeRangeMode = 'original'"
                />
              </div>
              <div class="choice-card-note-stack">
                <p class="choice-card-inline-note">{{ retryTranscodeSummary }}</p>
              </div>
            </section>
          </div>
          <div class="modal-actions">
            <UButton icon="i-lucide-x" :label="t('common.cancel')" color="warning" variant="outline" size="sm" :disabled="retrySubmittingMode !== null" @click="closeRetryConfirm" />
          </div>
        </div>
      </template>
    </UModal>

    <section class="detail-content">
      <div class="detail-grid">
        <UCard variant="subtle" :ui="{ root: 'detail-section detail-full', body: 'detail-card-body' }">
          <h2 class="detail-card-title">{{ t('jobDetails.projectDetails') }}</h2>
          <div class="detail-job-meta-stack">
            <div class="detail-job-meta">
              <span class="job-meta-item detail-job-meta-item">
                <span class="job-meta-label">{{ t('jobDetails.stats.format') }}</span>
                <strong>{{ displayOutputModeLabel }}</strong>
              </span>
              <span class="job-meta-item detail-job-meta-item">
                <span class="job-meta-label">{{ t('jobDetails.stats.specs') }}</span>
                <strong>{{ specsLabel }}</strong>
              </span>
              <span class="job-meta-item detail-job-meta-item">
                <span class="job-meta-label">{{ t('jobDetails.stats.frameRange') }}</span>
                <strong>{{ originalFrameRangeLabel }} ({{ t('jobDetails.stats.totalFrames', { count: originalFrameTotal }) }})</strong>
              </span>
              <span v-if="showCurrentExecutionRange" class="job-meta-item detail-job-meta-item">
                <span class="job-meta-label">{{ t('jobDetails.stats.currentExecution') }}</span>
                <strong>{{ currentExecutionRangeLabel }} ({{ t('jobDetails.stats.totalFrames', { count: currentExecutionTotal }) }})</strong>
              </span>
              <span class="job-meta-item detail-job-meta-item">
                <span class="job-meta-label">{{ t('jobDetails.stats.blenderVersion') }}</span>
                <strong>{{ blenderVersion }}</strong>
              </span>
              <span v-if="crashCount" class="job-meta-item detail-job-meta-item">
                <span class="job-meta-label">{{ t('jobDetails.stats.crashRecovery') }}</span>
                <strong>{{ t('jobDetails.stats.crashTimes', { count: crashCount }) }}</strong>
              </span>
              <span class="job-meta-item detail-job-meta-item">
                <span class="job-meta-label">{{ t('jobDetails.stats.started') }}</span>
                <strong>{{ formatTime(job.startedAt ?? job.createdAt) }}</strong>
              </span>
              <span class="job-meta-item detail-job-meta-item">
                <span class="job-meta-label">{{ t('jobDetails.stats.finished') }}</span>
                <strong>{{ job.finishedAt ? formatTime(job.finishedAt) : '—' }}</strong>
              </span>
              <span class="job-meta-item detail-job-meta-item">
                <span class="job-meta-label">{{ t('jobDetails.stats.duration') }}</span>
                <strong>{{ duration }}</strong>
              </span>
            </div>
            <div v-if="job.status === 'running'" class="detail-progress-meta">
              <span class="job-meta-label">{{ t('jobDetails.stats.renderProgress') }}</span>
              <RenderProgress
                class="detail-render-progress"
                :frame="job.currentFrame ?? 0"
                :total-frames="job.totalFrames ?? (job.frameEnd - job.frameStart + 1)"
                :warming-up="jobsStore.isJobWarmingUp(job.id)"
                :time-elapsed="job.timeElapsed ?? undefined"
                :remaining-secs="job.remainingSecs"
              />
            </div>
          </div>
        </UCard>

        <UCard variant="subtle" :ui="{ root: 'detail-section detail-full', body: 'detail-card-body' }">
          <h2 class="detail-card-title">{{ t('jobDetails.filePaths') }}</h2>
          <div class="detail-info-stack">
            <section class="detail-info-item">
              <div class="detail-path-chip">
                <span class="detail-path-label">{{ t('jobDetails.blendFile') }}</span>
                <button class="detail-path-text" type="button" :title="job.blendFile" @click="openPath(job.blendFile)">
                  {{ job.blendFile }}
                </button>
                <UTooltip :text="t('jobDetails.copyPath')" :content="{ side: 'top', sideOffset: 6 }">
                  <UButton icon="i-lucide-copy" color="neutral" variant="ghost" size="xs" square @click="copyPath(job.blendFile)" />
                </UTooltip>
              </div>
            </section>
            <section class="detail-info-item">
              <div class="detail-path-stack">
                <div class="detail-path-chip">
                  <span class="detail-path-label">{{ t('jobDetails.outputPath') }}</span>
                  <button class="detail-path-text" type="button" :title="job.outputPath" @click="openPath(resolveOutputDirectory(job.outputPath))">
                    {{ job.outputPath }}
                  </button>
                  <UTooltip :text="t('jobDetails.copyPath')" :content="{ side: 'top', sideOffset: 6 }">
                    <UButton icon="i-lucide-copy" color="neutral" variant="ghost" size="xs" square @click="copyPath(job.outputPath)" />
                  </UTooltip>
                </div>
              </div>
            </section>
          </div>
        </UCard>

        <UCard variant="subtle" :ui="{ root: 'detail-section detail-full preview-card', body: 'detail-card-body' }">
        <div class="preview-card-head">
          <h2 class="detail-card-title preview-card-title">{{ previewCardTitle }}</h2>
          <div v-if="hasFramePreview && hasVideoPreview" class="preview-switch">
            <UButton
              :icon="activePreviewTab === 'video' ? 'i-lucide-image' : 'i-lucide-clapperboard'"
              :label="activePreviewTab === 'video' ? t('jobDetails.preview.framePreview') : t('jobDetails.preview.videoPreview')"
              color="neutral"
              variant="subtle"
              size="sm"
              @click="togglePreviewTab"
            />
          </div>
        </div>
        <template v-if="activePreviewTab === 'video' && hasVideoPreview">
          <div class="surface-panel preview-thumb-wrap preview-video-wrap">
            <VideoPreviewPlayer
              :src="videoPreviewUrl!"
              :poster="videoPreviewPosterUrl"
              :title="videoPreviewTitle"
            />
          </div>
        </template>
        <template v-else-if="hasFramePreview">
          <div
            class="surface-panel preview-thumb-wrap"
            :class="{ 'preview-thumb-clickable': !!previewUrl }"
            :style="previewStyle"
            @click="previewUrl && (lightboxOpen = true)"
          >
            <img v-if="previewUrl" :src="previewUrl" class="preview-thumb" alt="last frame" />
            <div v-else class="preview-thumb-empty">
              <UIcon name="i-lucide-image" class="preview-thumb-icon" />
              <span>{{ previewPlaceholderText }}</span>
            </div>
            <UBadge
              v-if="previewFrame && previewUrl"
              :label="t('jobDetails.preview.frameBadge', { frame: previewFrame })"
              color="neutral"
              variant="subtle"
              class="preview-frame-label"
            />
          </div>
        </template>
        <div v-else class="surface-panel preview-thumb-wrap">
          <div class="preview-thumb-empty">
            <UIcon :name="hasVideoPreview ? 'i-lucide-video' : 'i-lucide-image-off'" class="preview-thumb-icon" />
            <span>{{ previewUnavailableText }}</span>
          </div>
        </div>
        </UCard>

        <UCard variant="subtle" :ui="{ root: 'detail-section detail-full log-section', body: 'detail-card-body' }">
          <div class="log-header">
            <h2 class="detail-card-title log-title">{{ t('jobDetails.logs.title') }}</h2>
            <div class="log-header-actions">
              <UButton
                :label="showAllLogs ? t('jobDetails.logs.latest') : t('jobDetails.logs.all')"
                :icon="showAllLogs ? 'i-lucide-clock' : 'i-lucide-layers'"
                color="neutral"
                variant="subtle"
                size="sm"
                :loading="logsLoading"
                @click="toggleLogScope"
              />
              <UButton
                v-if="logSummary?.directory"
                icon="i-lucide-folder-open"
                :label="t('jobDetails.logs.openDirectory')"
                color="neutral"
                variant="subtle"
                size="sm"
                @click="openPath(logSummary.directory)"
              />
            </div>
          </div>
          <div class="log-surface">
            <div class="log-panel" ref="logEl" @scroll="onLogScroll">
              <span v-if="logLines.length === 0" class="log-empty">
                {{ job.status === 'pending' ? t('jobDetails.logs.waiting') : t('jobDetails.logs.empty') }}
              </span>
              <div v-for="(entry, i) in displayJobLogs" :key="i" class="log-line">
                <div class="log-line-inner" :class="{ 'log-line-inner-no-timestamp': !entry.timestamp }">
                  <span v-if="entry.timestamp" class="log-line-timestamp">{{ entry.timestamp }}</span>
                  <span class="log-line-text">{{ entry.content || '\u00A0' }}</span>
                </div>
              </div>
            </div>
          </div>
        </UCard>
      </div>

      <p v-if="retryActionError" class="form-error">{{ retryActionError }}</p>
    </section>

    <UModal v-model:open="lightboxOpen" :close="false" :ui="{ content: 'preview-lightbox' }">
      <template #body>
        <div @click="lightboxOpen = false">
          <div @click.stop>
          <img :src="previewUrl!" class="preview-lightbox-img" alt="frame preview" />
          </div>
        </div>
      </template>
    </UModal>
  </div>

  <div v-else class="empty">{{ t('jobDetails.notFound') }}</div>
</template>

<script setup lang="ts">
import { convertFileSrc } from '@tauri-apps/api/core'
import type { DropdownMenuItem } from '@nuxt/ui'
import type { AddFfmpegJobPayload, JobLogSummary, RenderJob, RenderJobTranscodeConfig } from '~/types'
import { JOB_STATUS_COLOR, useJobStatusLabel } from '~/composables/useJobStatus'
import { RENDER_QUEUE_ORDER_HIDDEN_STATUSES, resolveQueueOrder, useQueueOrderLabel } from '~/composables/useQueueOrder'
import { buildTranscodeOutputPath, normalizeTranscodeDirectory, splitTranscodeOutputPath } from '~/composables/useTranscodeConfig'
import { useDateFormatters } from '~/utils/date-format'
import { parseLogLine } from '~/utils/log-line'
import { resolveOutputDirectory, resolvePathBaseName } from '~/utils/output-path'
import { captureVideoPoster } from '~/utils/video-preview'

const route = useRoute()
const router = useRouter()
const toast = useToast()
const jobsStore = useJobsStore()
const transcodeStore = useTranscodeStore()
const { t } = useI18n()
const { formatTimestamp } = useDateFormatters()

const settingsStore = useSettingsStore()

const { openPath, getLastRenderedFrame, getJobLogSummary, getJobLogs, updateJobPreviewDimensions, previewOutputPathTemplate, pathExists } = useTauri()
const { onProgress, onJobUpdated, onLog, onFfmpegJobUpdated } = useRenderEvents()
const statusLabel = useJobStatusLabel()
const queueOrderLabel = useQueueOrderLabel()

const jobId = computed(() => route.params.id as string)
const job = computed(() => jobsStore.jobs.find((j) => j.id === jobId.value))
const jobLogs = computed(() => jobsStore.getJobLogs(jobId.value))
const showAllLogs = ref(false)
const logsLoading = ref(false)
const allLogsLoaded = ref(false)
const allLogLines = ref<string[]>([])
const isQuickMp4Job = computed(() => job.value?.renderMode === 'quick_mp4')
const logLines = computed(() =>
  showAllLogs.value ? allLogLines.value : jobLogs.value,
)
const displayJobLogs = computed(() => logLines.value.map(line => parseLogLine(line)))
const relatedFfmpegJobs = computed(() => transcodeStore.getRelatedJobs(jobId.value))
const completedRelatedFfmpegJobs = computed(() =>
  relatedFfmpegJobs.value.filter(entry => entry.status === 'done'),
)
const primaryTranscodeJob = computed(() =>
  relatedFfmpegJobs.value.find(entry => entry.status === 'running')
  ?? relatedFfmpegJobs.value.find(entry => entry.status === 'pending')
  ?? relatedFfmpegJobs.value[0]
  ?? null,
)
const detailBreadcrumbItems = computed(() => {
  const currentJob = job.value
  if (!currentJob) return []
  return [
    { label: t('renderQueue.title'), to: '/' },
    { label: `#${currentJob.jobNumber} ${currentJob.name}` },
  ]
})
const statusBadgeColor = computed(() => JOB_STATUS_COLOR[job.value?.status ?? 'pending'] ?? 'neutral')
const queueOrder = computed(() => resolveQueueOrder(jobsStore.jobs, job.value, RENDER_QUEUE_ORDER_HIDDEN_STATUSES))
const orderBadgeLabel = computed(() => queueOrderLabel(queueOrder.value))
const metadataDialogOpen = ref(false)
const metadataJob = computed(() => job.value ?? null)
const transcodeModalOpen = ref(false)
const transcodeSettingsModalOpen = ref(false)
const resolvedBaseTranscodeOutputPath = ref('')
const blenderVersion = computed(() => {
  const exe = job.value?.blenderExecutable
  if (!exe) return '—'
  const match = settingsStore.blenderVersions.find((b) => b.executable === exe)
  return match ? `Blender ${match.version}` : exe
})
const originalFrameRangeLabel = computed(() => {
  const currentJob = job.value
  if (!currentJob) return '—'
  return `${currentJob.originalFrameStart} – ${currentJob.originalFrameEnd}`
})
const originalFrameTotal = computed(() => {
  const currentJob = job.value
  if (!currentJob) return 0
  return currentJob.originalFrameEnd - currentJob.originalFrameStart + 1
})
const currentExecutionRangeLabel = computed(() => {
  const currentJob = job.value
  if (!currentJob) return '—'
  return `${currentJob.frameStart} – ${currentJob.frameEnd}`
})
const currentExecutionTotal = computed(() => {
  const currentJob = job.value
  if (!currentJob) return 0
  return currentJob.frameEnd - currentJob.frameStart + 1
})
const showCurrentExecutionRange = computed(() => {
  const currentJob = job.value
  if (!currentJob) return false
  return currentJob.originalFrameStart !== currentJob.frameStart || currentJob.originalFrameEnd !== currentJob.frameEnd
})
const displayOutputModeLabel = computed(() => {
  if (isQuickMp4Job.value) return t('renderQueue.outputMode.QUICK_MP4')
  if (job.value?.outputFormat === 'PNG') return t('renderQueue.outputMode.PNG')
  if (job.value?.outputFormat === 'OPEN_EXR' || job.value?.outputFormat === 'EXR') return t('renderQueue.outputMode.OPEN_EXR')
  return job.value?.outputFormat ?? '—'
})
const resolutionLabel = computed(() => {
  const width = job.value?.previewWidth
  const height = job.value?.previewHeight
  if (!width || !height || width <= 0 || height <= 0) return '—'
  return `${width}×${height}`
})
const fpsLabel = computed(() => {
  const fps = job.value?.fps
  if (!fps || fps <= 0) return '—'
  return Number.isInteger(fps) ? `${fps}` : fps.toFixed(3).replace(/0+$/, '').replace(/\.$/, '')
})
const specsLabel = computed(() => {
  const parts = [resolutionLabel.value, fpsLabel.value === '—' ? '—' : `${fpsLabel.value} FPS`].filter(part => part !== '—')
  return parts.length ? parts.join(' · ') : '—'
})

function deriveRenderSequenceDirectory(outputPath: string) {
  const normalized = outputPath.replace(/\\/g, '/')
  const slashIndex = normalized.lastIndexOf('/')
  return slashIndex >= 0 ? outputPath.slice(0, slashIndex) : outputPath
}

function getEffectiveTranscodeFrameRange(currentJob: RenderJob) {
  return {
    frameStart: currentJob.transcodeFrameStartOverride ?? currentJob.frameStart,
    frameEnd: currentJob.transcodeFrameEndOverride ?? currentJob.frameEnd,
  }
}

async function refreshResolvedBaseTranscodeOutputPath() {
  const currentJob = job.value
  if (!currentJob || currentJob.renderMode === 'quick_mp4') {
    resolvedBaseTranscodeOutputPath.value = ''
    return
  }

  const range = getEffectiveTranscodeFrameRange(currentJob)

  try {
    const preview = await previewOutputPathTemplate({
      kind: 'blender-ffmpeg',
      template: settingsStore.settings.blenderTranscodeOutputPathTemplate,
      blend_file: currentJob.blendFile,
      frame_start: range.frameStart,
      frame_end: range.frameEnd,
    })
    resolvedBaseTranscodeOutputPath.value = preview.resolvedPath || ''
  } catch {
    resolvedBaseTranscodeOutputPath.value = ''
  }
}

const baseTranscodeConfig = computed<RenderJobTranscodeConfig | null>(() => {
  const currentJob = job.value
  if (!currentJob) return null

  const fallbackOutputPath = buildTranscodeOutputPath(
    normalizeTranscodeDirectory(deriveRenderSequenceDirectory(currentJob.outputPath)),
    currentJob.name,
  )
  const outputPath = resolvedBaseTranscodeOutputPath.value || fallbackOutputPath
  const split = splitTranscodeOutputPath(outputPath)

  return {
    name: currentJob.name,
    fps: Math.max(1, Math.round(currentJob.fps && currentJob.fps > 0 ? currentJob.fps : 30)),
    outputPath,
    outputDir: split.outputDir,
    outputStem: split.outputStem,
    crf: settingsStore.settings.transcodeCrf,
    preset: settingsStore.settings.transcodePreset,
  }
})
const effectiveTranscodeConfig = computed<RenderJobTranscodeConfig | null>(() => {
  const currentJob = job.value
  const base = baseTranscodeConfig.value
  if (!currentJob || !base) return null

  const outputPath = currentJob.transcodeOutputPathOverride || base.outputPath
  const split = splitTranscodeOutputPath(outputPath)

  return {
    name: currentJob.transcodeNameOverride || base.name,
    fps: Math.max(1, Math.round(currentJob.transcodeFpsOverride && currentJob.transcodeFpsOverride > 0 ? currentJob.transcodeFpsOverride : base.fps)),
    outputPath,
    outputDir: split.outputDir,
    outputStem: split.outputStem,
    crf: currentJob.transcodeCrfOverride ?? base.crf,
    preset: currentJob.transcodePresetOverride || base.preset,
  }
})
const transcodeSupported = computed(() => {
  const format = job.value?.outputFormat
  return Boolean(job.value) && !isQuickMp4Job.value && format !== 'OPEN_EXR' && format !== 'EXR'
})

watch(
  () => [
    job.value?.id,
    job.value?.blendFile,
    job.value?.frameStart,
    job.value?.frameEnd,
    settingsStore.settings.blenderTranscodeOutputPathTemplate,
  ] as const,
  () => {
    void refreshResolvedBaseTranscodeOutputPath()
  },
  { immediate: true },
)

const autoTranscodeEnabled = computed(() => transcodeSupported.value && Boolean(job.value?.autoTranscodeMp4))
const transcodePrimaryAction = computed(() => {
  const currentJob = job.value
  const currentTranscodeJob = primaryTranscodeJob.value
  if (!currentJob) {
    return {
      label: t('jobDetails.transcode.submit'),
      icon: 'i-lucide-film',
      color: 'neutral' as const,
      loading: false,
      disabled: true,
      spin: false,
    }
  }

  if (!currentTranscodeJob) {
    if (!transcodeSupported.value) {
      return {
        label: t('jobDetails.transcode.exrDisabled'),
        icon: 'i-lucide-ban',
        color: 'neutral' as const,
        loading: false,
        disabled: true,
        spin: false,
      }
    }

    return {
      label: t('jobDetails.transcode.submit'),
      icon: 'i-lucide-film',
      color: 'neutral' as const,
      loading: false,
      disabled: currentJob.status === 'running',
      spin: false,
    }
  }

  const statusMap = {
    pending: { label: t('jobDetails.transcode.pending'), icon: 'i-lucide-loader-circle', color: 'warning' as const, spin: true },
    running: { label: t('jobDetails.transcode.running'), icon: 'i-lucide-loader-circle', color: 'info' as const, spin: true },
    done: { label: t('jobDetails.transcode.view'), icon: 'i-lucide-circle-check-big', color: 'success' as const, spin: false },
    failed: { label: t('jobDetails.transcode.view'), icon: 'i-lucide-triangle-alert', color: 'error' as const, spin: false },
    cancelled: { label: t('jobDetails.transcode.view'), icon: 'i-lucide-square', color: 'warning' as const, spin: false },
  }[currentTranscodeJob.status]

  return {
    label: statusMap.label,
    icon: statusMap.icon,
    color: statusMap.color,
    loading: false,
    disabled: false,
    spin: statusMap.spin,
  }
})

function openMetadataDialog() {
  if (!job.value) return
  metadataDialogOpen.value = true
}

async function copyPath(path: string) {
  if (!path) return
  try {
    await navigator.clipboard.writeText(path)
    toast.add({ title: t('jobDetails.copy.success'), color: 'success' })
  } catch (error) {
    toast.add({
      title: t('jobDetails.copy.failed'),
      description: error instanceof Error ? error.message : String(error),
      color: 'error',
    })
  }
}

function buildJobContextMenuItems(currentJob: RenderJob) {
  return [
    {
      label: t('jobDetails.actions.editMetadata'),
      icon: 'i-lucide-notebook-pen',
      onSelect: () => openMetadataDialog(),
    },
  ]
}

const updatingAutoTranscode = ref(false)
const logSummary = ref<JobLogSummary | null>(null)
const detailUnlisteners: Array<() => void> = []
let logSummaryTimer: ReturnType<typeof setTimeout> | null = null
const shadowRecoveryToast = useShadowRecoveryToast()

const crashCount = computed(() => job.value?.crashCount ?? 0)

const transcodeActionItems = computed<DropdownMenuItem[][]>(() => {
  const items: DropdownMenuItem[][] = [[
    {
      slot: 'auto-transcode',
      label: t('jobDetails.transcode.autoAfterRender'),
      icon: 'i-lucide-clapperboard',
      loading: updatingAutoTranscode.value,
      disabled: updatingAutoTranscode.value || !transcodeSupported.value,
      onSelect: (event: Event) => {
        event.preventDefault()
        void handleAutoTranscodeToggle(!autoTranscodeEnabled.value)
      },
    },
  ], [
    {
      label: t('jobDetails.transcode.settings'),
      icon: 'i-lucide-sliders',
      disabled: !job.value || !transcodeSupported.value,
      onSelect: () => {
        transcodeSettingsModalOpen.value = true
      },
    },
    {
      label: t('jobDetails.transcode.submitNow'),
      icon: 'i-lucide-film',
      disabled: job.value?.status === 'running' || !transcodeSupported.value,
      onSelect: () => {
        void submitTranscodeForJob()
      },
    },
  ], [
    {
      label: t('jobDetails.transcode.goToQueue'),
      icon: 'i-lucide-list-video',
      onSelect: () => router.push('/transcode'),
    },
  ]]

  if (primaryTranscodeJob.value) {
    items.push([
      {
        label: t('jobDetails.transcode.viewJob'),
        icon: 'i-lucide-file-text',
        onSelect: () => router.push(`/transcode/${primaryTranscodeJob.value?.id}`),
      },
    ])
  }

  return items
})

// ── Frame preview ──────────────────────────────────────────────────────────
const activePreviewTab = ref<'frame' | 'video'>('frame')
const previewUrl = ref<string | null>(null)
const previewFrame = ref<number | null>(null)
const previewAspect = ref<string | null>(null)
const lightboxOpen = ref(false)
const videoPreviewUrl = ref<string | null>(null)
const videoPreviewPosterUrl = ref<string | null>(null)
const videoPreviewSourcePath = ref<string | null>(null)
let videoPreviewPosterToken = 0
const hasFramePreview = computed(() =>
  Boolean(job.value) && !isQuickMp4Job.value && job.value?.outputFormat !== 'OPEN_EXR' && job.value?.outputFormat !== 'EXR',
)
const hasVideoPreview = computed(() => Boolean(videoPreviewUrl.value))
const videoPreviewTitle = computed(() =>
  resolvePathBaseName(videoPreviewSourcePath.value) || job.value?.name || t('jobDetails.preview.videoPreview'),
)
const previewCardTitle = computed(() => {
  if (isQuickMp4Job.value || (activePreviewTab.value === 'video' && hasVideoPreview.value)) return t('jobDetails.preview.videoPreview')
  return t('jobDetails.preview.framePreview')
})
const previewPlaceholderText = computed(() => {
  if (isQuickMp4Job.value) return t('jobDetails.preview.quickMp4UsesVideo')
  if (job.value?.outputFormat === 'OPEN_EXR' || job.value?.outputFormat === 'EXR') return t('jobDetails.preview.exrUnsupported')
  return t('jobDetails.preview.noRenderedFrames')
})
const previewUnavailableText = computed(() => {
  if (isQuickMp4Job.value) {
    return job.value?.status === 'done'
      ? t('jobDetails.preview.videoUnavailableDone')
      : t('jobDetails.preview.videoUnavailablePending')
  }
  if (completedRelatedFfmpegJobs.value.length > 0) {
    return t('jobDetails.preview.transcodeVideoUnavailable')
  }
  return t('jobDetails.preview.noPlayableVideo')
})
const previewStyle = computed(() =>
  previewAspect.value ? { '--preview-aspect': previewAspect.value } : undefined,
)
const previewFrameEnd = computed(() => {
  const j = job.value
  if (!j) return null
  if (j.lastRenderedFrame != null) {
    return Math.min(j.frameEnd, Math.max(j.frameStart, j.lastRenderedFrame))
  }
  if (j.status !== 'running') return j.frameEnd
  const progressed = Math.max(j.currentFrame ?? 0, 0)
  const capped = j.frameStart + progressed - 1
  return Math.min(j.frameEnd, Math.max(j.frameStart, capped))
})

function aspectFromDimensions(width: number | null | undefined, height: number | null | undefined) {
  if (!width || !height || width <= 0 || height <= 0) return null
  return `${width} / ${height}`
}

function applyStoredPreviewAspect() {
  previewAspect.value = aspectFromDimensions(job.value?.previewWidth, job.value?.previewHeight)
}

async function refreshVideoPreview() {
  const currentJob = job.value
  const token = ++videoPreviewPosterToken

  const candidates: string[] = []
  if (currentJob?.renderMode === 'quick_mp4' && currentJob.status === 'done') {
    candidates.push(currentJob.outputPath)
  } else {
    candidates.push(...completedRelatedFfmpegJobs.value.map(entry => entry.outputPath))
  }

  let resolvedPath: string | null = null
  for (const candidate of candidates) {
    if (!candidate) continue
    try {
      if (await pathExists(candidate)) {
        resolvedPath = candidate
        break
      }
    } catch {
      // Ignore transient filesystem checks and keep looking.
    }
  }

  if (token !== videoPreviewPosterToken) return

  if (!resolvedPath) {
    videoPreviewSourcePath.value = null
    videoPreviewUrl.value = null
    videoPreviewPosterUrl.value = null
    return
  }

  const url = `${convertFileSrc(resolvedPath)}?t=${Date.now()}`
  videoPreviewSourcePath.value = resolvedPath
  videoPreviewUrl.value = url
  videoPreviewPosterUrl.value = null

  if (currentJob?.previewImagePath) {
    try {
      if (await pathExists(currentJob.previewImagePath)) {
        if (token !== videoPreviewPosterToken) return
        videoPreviewPosterUrl.value = `${convertFileSrc(currentJob.previewImagePath)}?t=${Date.now()}`
        return
      }
    } catch {
      // Fall through to runtime capture when the stored preview is unavailable.
    }
  }

  const poster = await captureVideoPoster(url).catch(() => null)
  if (token !== videoPreviewPosterToken) return
  videoPreviewPosterUrl.value = poster?.dataUrl ?? null
  if (poster) {
    void syncStoredPreviewDimensions(poster.width, poster.height)
  }
}

async function preloadPreview(url: string) {
  return new Promise<{ width: number, height: number }>((resolve, reject) => {
    const img = new Image()
    img.onload = () => resolve({ width: img.naturalWidth, height: img.naturalHeight })
    img.onerror = () => reject(new Error('preview load failed'))
    img.src = url
  })
}

async function syncStoredPreviewDimensions(width: number, height: number) {
  const j = job.value
  if (!j) return
  const currentKey = j.previewWidth && j.previewHeight ? `${j.previewWidth}x${j.previewHeight}` : null
  const nextKey = `${width}x${height}`
  if (currentKey === nextKey) return

  try {
    const updated = await updateJobPreviewDimensions(j.id, width, height)
    jobsStore.mergeJob(updated)
  } catch {
    // Ignore persistence failures; preview display should still work.
  }
}

async function refreshPreview() {
  const j = job.value
  if (!j || j.renderMode === 'quick_mp4' || j.outputFormat === 'OPEN_EXR' || j.outputFormat === 'EXR') {
    previewUrl.value = null
    previewFrame.value = null
    applyStoredPreviewAspect()
    return
  }
  try {
    const path = await getLastRenderedFrame(
      j.outputPath,
      j.outputFormat,
      j.frameStart,
      previewFrameEnd.value ?? j.frameEnd,
    )
    if (!path) {
      previewUrl.value = null
      previewFrame.value = null
      applyStoredPreviewAspect()
      return
    }
    const url = `${convertFileSrc(path)}?t=${Date.now()}`
    const { width, height } = await preloadPreview(url)
    previewAspect.value = `${width} / ${height}`
    previewUrl.value = url
    void syncStoredPreviewDimensions(width, height)
    // Extract frame number from filename
    const match = path.match(/(\d+)\.[^.]+$/)
    const frameToken = match?.[1]
    previewFrame.value = frameToken ? parseInt(frameToken) : null
  } catch {
    previewUrl.value = null
    previewFrame.value = null
    applyStoredPreviewAspect()
  }
}

watch(
  () => [
    job.value?.id,
    job.value?.status,
    job.value?.renderMode,
    job.value?.outputPath,
    job.value?.previewImagePath,
    ...completedRelatedFfmpegJobs.value.map(entry => `${entry.id}:${entry.status}:${entry.outputPath}:${entry.finishedAt ?? 0}`),
  ] as const,
  () => {
    void refreshVideoPreview()
  },
  { immediate: true },
)

watch(
  [hasFramePreview, hasVideoPreview],
  ([frameAvailable, videoAvailable]) => {
    if (videoAvailable && (!frameAvailable || activePreviewTab.value === 'video')) {
      activePreviewTab.value = 'video'
      return
    }

    if (frameAvailable) {
      activePreviewTab.value = 'frame'
    }
  },
  { immediate: true },
)

function togglePreviewTab() {
  if (!hasFramePreview.value || !hasVideoPreview.value) return
  activePreviewTab.value = activePreviewTab.value === 'video' ? 'frame' : 'video'
}

watch(
  () => jobLogs.value.length,
  () => { if (job.value?.status === 'running') refreshPreview() },
)

watch(
  () => job.value?.status,
  (newStatus, oldStatus) => {
    if (oldStatus === 'running' && newStatus !== 'running') refreshPreview()
  },
)

watch(
  () => [job.value?.previewWidth, job.value?.previewHeight] as const,
  () => {
    if (!previewUrl.value) {
      applyStoredPreviewAspect()
    }
  },
  { immediate: true },
)

// ─────────────────────────────────────────────────────────────────────────────

const logEl = ref<HTMLDivElement | null>(null)
const pinToBottom = ref(true)

watch(
  () => logLines.value.length,
  async () => {
    if (!pinToBottom.value) return
    await nextTick()
    if (logEl.value) logEl.value.scrollTop = logEl.value.scrollHeight
  },
)

async function toggleLogScope() {
  const nextShowAll = !showAllLogs.value
  showAllLogs.value = nextShowAll
  if (!nextShowAll || allLogsLoaded.value) return

  logsLoading.value = true
  try {
    allLogLines.value = await getJobLogs(jobId.value)
    allLogsLoaded.value = true
  } catch (error) {
    showAllLogs.value = false
    toast.add({
      title: t('jobDetails.logs.loadAllFailed'),
      description: error instanceof Error ? error.message : String(error),
      color: 'error',
    })
  } finally {
    logsLoading.value = false
  }
}

function scheduleLogSummaryRefresh(delay = 180) {
  if (logSummaryTimer) clearTimeout(logSummaryTimer)
  logSummaryTimer = setTimeout(() => {
    void refreshLogSummary()
  }, delay)
}

async function refreshLogSummary() {
  try {
    logSummary.value = await getJobLogSummary(jobId.value)
  } catch {
    logSummary.value = null
  }
}

watch(
  () => jobLogs.value.length,
  () => {
    scheduleLogSummaryRefresh()
  },
)

watch(
  () => job.value?.status,
  () => {
    scheduleLogSummaryRefresh(40)
  },
)

function onLogScroll() {
  const el = logEl.value
  if (!el) return
  pinToBottom.value = el.scrollTop + el.clientHeight >= el.scrollHeight - 40
}

const formatTime = formatTimestamp

const duration = computed(() => {
  const j = job.value
  if (!j?.startedAt) return '—'
  const ms = (j.finishedAt ?? Date.now()) - j.startedAt
  const secs = Math.round(ms / 1000)
  if (secs < 60) return `${secs}s`
  const m = Math.floor(secs / 60)
  const s = secs % 60
  if (m < 60) return `${m}m ${s}s`
  return `${Math.floor(m / 60)}h ${m % 60}m ${s}s`
})

onMounted(async () => {
  await Promise.all([
    settingsStore.load(),
    jobsStore.fetchJobs(),
    transcodeStore.fetchFfmpegJobs(),
  ])
  detailUnlisteners.push(await onProgress((event) => {
    jobsStore.applyProgress(event)
  }))
  detailUnlisteners.push(await onJobUpdated((event) => {
    jobsStore.applyJobUpdate(event)
  }))
  detailUnlisteners.push(await onLog((event) => {
    if (event.jobId !== jobId.value || !allLogsLoaded.value) return
    if (allLogLines.value.at(-1) === event.line) return
    allLogLines.value = [...allLogLines.value, event.line]
  }))
  detailUnlisteners.push(await onFfmpegJobUpdated((event) => {
    transcodeStore.applyFfmpegJobUpdate(event)
  }))
  await Promise.all([
    jobsStore.loadJobLogs(jobId.value),
    refreshLogSummary(),
  ])
  if (job.value) {
    shadowRecoveryToast.handleExistingLogs(job.value, jobLogs.value)
  }
  await refreshPreview()
})

onUnmounted(() => {
  for (const unlisten of detailUnlisteners) {
    unlisten()
  }
  cancelRetryCloseCleanup()
  if (logSummaryTimer) clearTimeout(logSummaryTimer)
})

const showDeleteConfirm = ref(false)
const {
  showRetryConfirm,
  retryIsQuickMp4,
  retryActionError,
  retrySubmittingMode,
  retryCustomStart,
  retryCustomEnd,
  retryFullRangePreviewMode,
  retryCustomPreviewMode,
  retryCustomInspectLoading,
  retryAutoTranscodeEnabled,
  retryTranscodeRangeMode,
  retryFullRangeLabel,
  retryFullRangeSummary,
  retryCustomActionDescription,
  retryCustomRangeSummary,
  retryOriginalTranscodeRangeLabel,
  retrySavedTranscodeRangeTitle,
  retryCurrentTargetRangeLabel,
  retryTranscodeSummary,
  cancelRetryCloseCleanup,
  handleRetry,
  closeRetryConfirm,
  clearRetryPreviewOnLeave,
  confirmRetryContinue,
  confirmRetryFromStart,
  confirmRetryCustomRange,
} = useJobRetry()

async function handleAutoTranscodeToggle(value: boolean) {
  const currentJob = job.value
  if (!currentJob || updatingAutoTranscode.value) return
  if (!transcodeSupported.value) return

  updatingAutoTranscode.value = true
  try {
    await jobsStore.updateJobTranscodeSettings({
      id: currentJob.id,
      auto_transcode_mp4: value,
      transcode_name_override: currentJob.transcodeNameOverride,
      transcode_fps_override: currentJob.transcodeFpsOverride,
      transcode_output_path_override: currentJob.transcodeOutputPathOverride,
      transcode_crf_override: currentJob.transcodeCrfOverride,
      transcode_preset_override: currentJob.transcodePresetOverride,
      transcode_frame_start_override: currentJob.transcodeFrameStartOverride,
      transcode_frame_end_override: currentJob.transcodeFrameEndOverride,
    })
  } finally {
    updatingAutoTranscode.value = false
  }
}

function handleAutoTranscodeSwitchUpdate(value: boolean) {
  void handleAutoTranscodeToggle(value)
}

async function submitTranscodeForJob() {
  const currentJob = job.value
  if (!currentJob || currentJob.status === 'running' || !transcodeSupported.value) return
  retryActionError.value = ''
  transcodeModalOpen.value = true
}

async function handleTranscodeSettingsSave(payload: {
  transcode_name_override: string | null
  transcode_fps_override: number | null
  transcode_output_path_override: string | null
  transcode_crf_override: number | null
  transcode_preset_override: string | null
}) {
  const currentJob = job.value
  if (!currentJob || !transcodeSupported.value) return

  try {
    await jobsStore.updateJobTranscodeSettings({
      id: currentJob.id,
      auto_transcode_mp4: currentJob.autoTranscodeMp4,
      ...payload,
      transcode_frame_start_override: currentJob.transcodeFrameStartOverride,
      transcode_frame_end_override: currentJob.transcodeFrameEndOverride,
    })
    transcodeSettingsModalOpen.value = false
  } catch (error) {
    retryActionError.value = error instanceof Error ? error.message : String(error)
  }
}

async function handleTranscodeSubmit(payload: AddFfmpegJobPayload) {
  try {
    const ffmpegJob = await transcodeStore.submitFfmpegJob(payload)
    transcodeModalOpen.value = false
    router.push(`/transcode/${ffmpegJob.id}`)
  } catch (error) {
    retryActionError.value = error instanceof Error ? error.message : String(error)
  }
}

async function handlePrimaryTranscodeAction() {
  if (!transcodeSupported.value) return
  if (primaryTranscodeJob.value) {
    await router.push(`/transcode/${primaryTranscodeJob.value.id}`)
    return
  }
  await submitTranscodeForJob()
}

async function removeAndBack() {
  const j = job.value
  if (!j) return
  await jobsStore.deleteJob(j.id)
  router.push('/')
}
</script>
