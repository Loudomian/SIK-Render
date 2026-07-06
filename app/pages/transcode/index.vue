<template>
  <div class="queue-page">
    <Transition name="drop-fade">
      <div v-if="isDragging && !draggedJobId" class="drop-overlay">
        <div class="drop-message">
          <UIcon name="i-lucide-folder-down" class="drop-icon" />
          <div class="drop-copy">
            <strong>{{ t('transcodeQueue.dragOverlay.title') }}</strong>
            <span>{{ t('transcodeQueue.dragOverlay.description') }}</span>
          </div>
        </div>
      </div>
    </Transition>

    <section class="queue-header">
      <section class="page-hero queue-hero">
        <div class="page-hero-copy">
          <div class="queue-heading-row">
            <div class="queue-heading-copy">
              <h1>{{ t('transcodeQueue.title') }}</h1>
              <p class="page-note">{{ t('transcodeQueue.description') }}</p>
            </div>
            <div class="queue-hero-actions-stack">
              <div class="page-hero-actions queue-hero-actions">
                <UFieldGroup size="md">
                  <UButton
                    icon="i-lucide-plus"
                    :label="t('transcodeQueue.newJob')"
                    color="primary"
                    variant="solid"
                    @click="openCreateModal"
                  />
                  <UDropdownMenu
                    :items="transcodeQueueActionItems"
                    arrow
                    :content="{ side: 'bottom', align: 'end', sideOffset: 8 }"
                  >
                    <UButton
                      icon="i-lucide-chevron-down"
                      color="neutral"
                      variant="outline"
                      square
                      :disabled="clearingCompletedTranscodeJobs"
                    />
                  </UDropdownMenu>
                </UFieldGroup>
              </div>
            </div>
          </div>
        </div>
      </section>

      <div class="queue-tabs-row surface-panel">
        <UTabs
          v-model="activeTab"
          :items="tabItems"
          :content="false"
          color="neutral"
          variant="pill"
          class="queue-tabs"
          :ui="{
            indicator: 'hidden',
            list: 'queue-tabs-list',
            trigger: 'queue-tab-trigger',
            label: 'queue-tab-label',
          }"
        />
      </div>
    </section>

    <section class="queue-content">
      <div v-if="transcodeStore.loading" class="loading">{{ t('common.loading') }}</div>

      <UCard
        v-else-if="transcodeStore.ffmpegJobs.length === 0"
        variant="subtle"
        class="empty-state"
        :ui="{ body: 'empty-state-body' }"
      >
        <div class="empty-state-icon">
          <UIcon name="i-lucide-clapperboard" />
        </div>
        <div class="empty-state-title">{{ t('transcodeQueue.empty.title') }}</div>
        <div class="empty-state-note">{{ t('transcodeQueue.empty.note') }}</div>
      </UCard>

      <UCard
        v-else-if="filteredJobs.length === 0"
        variant="subtle"
        class="empty-state queue-empty-state"
        :class="emptyTabToneClass"
        :ui="{ body: 'empty-state-body' }"
      >
        <div class="empty-state-icon">
          <UIcon name="i-lucide-filter" />
        </div>
        <div class="empty-state-title">{{ emptyTabTitle }}</div>
        <div class="empty-state-note">{{ emptyTabNote }}</div>
      </UCard>

      <TransitionGroup v-else name="job-list" tag="div" class="job-list">
        <div
          v-for="job in filteredJobs"
          :key="job.id"
          :data-job-id="job.id"
          class="job-list-item"
          :class="{
            'job-list-item-draggable': canDragJob(job),
            'job-list-item-dragging': draggedJobId === job.id,
            'job-list-item-drop-before': dropTargetJobId === job.id && dropPosition === 'before',
            'job-list-item-drop-after': dropTargetJobId === job.id && dropPosition === 'after',
          }"
          @click="handleCardClick(job, $event)"
          @pointerdown="handlePointerDown(job, $event)"
        >
          <FfmpegJobCard
            :job="job"
            @cancel="handleCancel(job.id)"
            @remove="deleteConfirmJob = job"
          />
        </div>
      </TransitionGroup>
    </section>

    <UModal
      :open="!!deleteConfirmJob"
      :close="false"
      :title="t('transcodeQueue.delete.title')"
      :ui="{ content: 'job-modal-content' }"
      @update:open="v => { if (!v) deleteConfirmJob = null }"
    >
      <template #body>
        <div class="modal-stack">
          <p class="modal-copy">
            {{ t('transcodeQueue.delete.message', { name: deleteConfirmJob?.name ?? '' }) }}
          </p>
          <div class="modal-actions">
            <UButton icon="i-lucide-x" :label="t('common.cancel')" color="warning" variant="outline" @click="deleteConfirmJob = null" />
            <UButton icon="i-lucide-trash-2" :label="t('common.delete')" color="error" variant="outline" @click="confirmDelete" />
          </div>
        </div>
      </template>
    </UModal>

    <UModal
      :open="showClearCompletedConfirm"
      :close="false"
      :title="t('transcodeQueue.clearCompleted.title')"
      :ui="{ content: 'job-modal-content' }"
      @update:open="v => { if (!v) showClearCompletedConfirm = false }"
    >
      <template #body>
        <div class="modal-stack">
          <p class="modal-copy">
            {{ t('transcodeQueue.clearCompleted.message', { count: doneJobs.length }) }}
          </p>
          <div class="modal-actions">
            <UButton
              icon="i-lucide-x"
              :label="t('common.cancel')"
              color="warning"
              variant="outline"
              :disabled="clearingCompletedTranscodeJobs"
              @click="showClearCompletedConfirm = false"
            />
            <UButton
              icon="i-lucide-trash-2"
              :label="t('transcodeQueue.actions.clearCompletedConfirm')"
              color="error"
              variant="outline"
              :loading="clearingCompletedTranscodeJobs"
              @click="confirmClearCompletedTranscodeJobs"
            />
          </div>
        </div>
      </template>
    </UModal>

    <TranscodeSubmitModal
      :open="transcodeModalOpen"
      :folder-path="activePendingFolder?.path"
      :folder-input-path="activePendingFolder?.inputPath"
      :folder-frame-start="activePendingFolder?.frameStart"
      :folder-frame-end="activePendingFolder?.frameEnd"
      :folder-name="activePendingFolder?.name"
      @submit="handleModalSubmit"
      @close="closePendingFolderModal"
      @update:open="transcodeModalOpen = $event"
    />
  </div>
</template>

<script setup lang="ts">
import { getCurrentWindow } from '@tauri-apps/api/window'
import type { DropdownMenuItem, TabsItem } from '@nuxt/ui'
import type { AddFfmpegJobPayload, FfmpegJob } from '~/types'

const router = useRouter()
const route = useRoute()
const toast = useToast()
const settingsStore = useSettingsStore()
const transcodeStore = useTranscodeStore()
const { t } = useI18n()
const { scanFolderFrameGroups } = useTauri()
const { onTranscodeProgress, onTranscodeLog, onFfmpegJobUpdated } = useRenderEvents()

const activeTab = ref<'all' | 'queue' | 'done' | 'error'>('all')
const isDragging = ref(false)
const transcodeModalOpen = ref(false)
const clearingCompletedTranscodeJobs = ref(false)
const showClearCompletedConfirm = ref(false)
const deleteConfirmJob = ref<FfmpegJob | null>(null)
const pendingFolderQueue = ref<Array<{
  path: string
  inputPath: string
  frameStart: number
  frameEnd: number
  name: string
}>>([])
const draggedJobId = ref<string | null>(null)
const dropTargetJobId = ref<string | null>(null)
const dropPosition = ref<'before' | 'after'>('before')
const reorderingQueue = ref(false)
const pointerDragging = ref(false)
const unlisteners: Array<() => void> = []
const activePendingFolder = computed(() => pendingFolderQueue.value[0] ?? null)
const queueJobs = computed(() =>
  transcodeStore.ffmpegJobs.filter(job => job.status === 'pending' || job.status === 'running'),
)
const doneJobs = computed(() =>
  transcodeStore.ffmpegJobs.filter(job => job.status === 'done'),
)
const errorJobs = computed(() =>
  transcodeStore.ffmpegJobs.filter(job => job.status === 'failed' || job.status === 'cancelled'),
)
const filteredJobs = computed(() => {
  switch (activeTab.value) {
    case 'queue':
      return queueJobs.value
    case 'done':
      return doneJobs.value
    case 'error':
      return errorJobs.value
    default:
      return transcodeStore.ffmpegJobs
  }
})
const emptyTabTitle = computed(() => {
  switch (activeTab.value) {
    case 'queue':
      return t('transcodeQueue.empty.queueTitle')
    case 'done':
      return t('transcodeQueue.empty.doneTitle')
    case 'error':
      return t('transcodeQueue.empty.errorTitle')
    default:
      return t('transcodeQueue.empty.allTitle')
  }
})
const emptyTabNote = computed(() => {
  switch (activeTab.value) {
    case 'queue':
      return t('transcodeQueue.empty.queueNote')
    case 'done':
      return t('transcodeQueue.empty.doneNote')
    case 'error':
      return t('transcodeQueue.empty.errorNote')
    default:
      return t('transcodeQueue.empty.allNote')
  }
})
const emptyTabToneClass = computed(() => {
  switch (activeTab.value) {
    case 'queue':
      return 'queue-empty-tone-queue'
    case 'done':
      return 'queue-empty-tone-done'
    case 'error':
      return 'queue-empty-tone-error'
    default:
      return 'queue-empty-tone-all'
  }
})
const transcodeQueueActionItems = computed<DropdownMenuItem[][]>(() => [[
  {
    label: t('transcodeQueue.actions.clearCompleted'),
    icon: 'i-lucide-trash-2',
    disabled: clearingCompletedTranscodeJobs.value || doneJobs.value.length === 0,
    onSelect: () => {
      showClearCompletedConfirm.value = true
    },
  },
]])
const tabItems = computed<TabsItem[]>(() => [
  {
    label: t('transcodeQueue.tabs.all'),
    value: 'all',
    badge: { label: String(transcodeStore.ffmpegJobs.length), color: 'neutral' as const, variant: 'subtle' as const },
    icon: 'i-lucide-layers',
    class: 'queue-tab-tone-all',
    ui: { trigger: 'queue-tab-tone-all' },
  },
  {
    label: t('transcodeQueue.tabs.queue'),
    value: 'queue',
    badge: { label: String(queueJobs.value.length), color: 'info' as const, variant: 'subtle' as const },
    icon: 'i-lucide-loader-circle',
    class: 'queue-tab-tone-queue',
    ui: { trigger: 'queue-tab-tone-queue' },
  },
  {
    label: t('transcodeQueue.tabs.done'),
    value: 'done',
    badge: { label: String(doneJobs.value.length), color: 'success' as const, variant: 'subtle' as const },
    icon: 'i-lucide-circle-check-big',
    class: 'queue-tab-tone-done',
    ui: { trigger: 'queue-tab-tone-done' },
  },
  {
    label: t('transcodeQueue.tabs.error'),
    value: 'error',
    badge: { label: String(errorJobs.value.length), color: 'warning' as const, variant: 'subtle' as const },
    icon: 'i-lucide-triangle-alert',
    class: 'queue-tab-tone-error',
    ui: { trigger: 'queue-tab-tone-error' },
  },
])

let dragPointerId: number | null = null
let dragStartX = 0
let dragStartY = 0
let pendingDragJobId: string | null = null
const DRAG_START_DISTANCE = 6
let lastClickJobId: string | null = null
let lastClickTime = 0
const DBLCLICK_MAX_DELAY = 400

async function addFolderToQueue(folderPath: string) {
  if (folderPath.toLowerCase().endsWith('.blend')) return

  try {
    const result = await scanFolderFrameGroups(folderPath)
    if (result.groups.length === 0) {
      toast.add({
        title: t('transcodeQueue.toast.noSequenceTitle'),
        description: t('transcodeQueue.toast.noSequenceDescription'),
        color: 'warning',
      })
      return
    }

    pendingFolderQueue.value = [
      ...pendingFolderQueue.value,
      ...result.groups.map(group => ({
        path: folderPath,
        inputPath: group.inputPath,
        frameStart: group.frameStart,
        frameEnd: group.frameEnd,
        name: group.name,
      })),
    ]
    transcodeModalOpen.value = true
  } catch (error) {
    toast.add({
      title: t('transcodeQueue.toast.createFailedTitle'),
      description: error instanceof Error ? error.message : String(error),
      color: 'error',
    })
  }
}

function openCreateModal() {
  pendingFolderQueue.value = []
  transcodeModalOpen.value = true
}

function advancePendingFolderQueue() {
  pendingFolderQueue.value = pendingFolderQueue.value.slice(1)
  transcodeModalOpen.value = pendingFolderQueue.value.length > 0
}

function closePendingFolderModal() {
  if (pendingFolderQueue.value.length > 0) {
    advancePendingFolderQueue()
  } else {
    transcodeModalOpen.value = false
  }
}

async function handleModalSubmit(payload: AddFfmpegJobPayload) {
  try {
    const job = await transcodeStore.submitFfmpegJob(payload)
    toast.add({
      title: t('transcodeQueue.toast.createdTitle'),
      description: `#${job.jobNumber} ${job.name}`,
      color: 'success',
    })
    if (pendingFolderQueue.value.length > 0) {
      advancePendingFolderQueue()
    } else {
      transcodeModalOpen.value = false
    }
  } catch (error) {
    toast.add({
      title: t('transcodeQueue.toast.createFailedTitle'),
      description: error instanceof Error ? error.message : String(error),
      color: 'error',
    })
  }
}

function canDragJob(job: FfmpegJob) {
  return job.status !== 'running' && !reorderingQueue.value
}

function clearDragState() {
  pointerDragging.value = false
  draggedJobId.value = null
  dropTargetJobId.value = null
  dropPosition.value = 'before'
  dragPointerId = null
  pendingDragJobId = null
  document.body.style.removeProperty('user-select')
}

function updateDropPosition(jobId: string, target: HTMLElement, clientY: number) {
  if (!draggedJobId.value || draggedJobId.value === jobId) return
  const job = transcodeStore.ffmpegJobs.find(entry => entry.id === jobId)
  if (!job || !canDragJob(job)) return
  const rect = target.getBoundingClientRect()
  dropTargetJobId.value = jobId
  dropPosition.value = clientY >= rect.top + rect.height / 2 ? 'after' : 'before'
}

function beginDrag(jobId: string) {
  pointerDragging.value = true
  draggedJobId.value = jobId
  dropTargetJobId.value = null
  dropPosition.value = 'before'
  document.body.style.userSelect = 'none'
}

function handlePointerMove(event: PointerEvent) {
  if (dragPointerId == null || event.pointerId !== dragPointerId || !pendingDragJobId) return

  if (!pointerDragging.value) {
    const dx = Math.abs(event.clientX - dragStartX)
    const dy = Math.abs(event.clientY - dragStartY)
    if (Math.max(dx, dy) < DRAG_START_DISTANCE) return
    beginDrag(pendingDragJobId)
  }

  const target = document.elementFromPoint(event.clientX, event.clientY)?.closest('.job-list-item') as HTMLElement | null
  const targetJobId = target?.dataset.jobId
  if (!target || !targetJobId) return
  updateDropPosition(targetJobId, target, event.clientY)
}

async function commitDrop() {
  if (!draggedJobId.value || !dropTargetJobId.value || draggedJobId.value === dropTargetJobId.value) {
    clearDragState()
    return
  }

  const draggedId = draggedJobId.value
  const allDraggableIds = transcodeStore.ffmpegJobs
    .filter(job => job.status !== 'running')
    .map(job => job.id)

  if (!allDraggableIds.includes(draggedId) || !allDraggableIds.includes(dropTargetJobId.value)) {
    clearDragState()
    return
  }

  const remaining = allDraggableIds.filter(id => id !== draggedId)
  const targetIndex = remaining.indexOf(dropTargetJobId.value)
  if (targetIndex === -1) {
    clearDragState()
    return
  }

  const insertIndex = dropPosition.value === 'after' ? targetIndex + 1 : targetIndex
  remaining.splice(insertIndex, 0, draggedId)

  if (remaining.join(',') === allDraggableIds.join(',')) {
    clearDragState()
    return
  }

  reorderingQueue.value = true
  try {
    await transcodeStore.reorderPendingJobs(remaining)
  } catch (error) {
    toast.add({
      title: t('transcodeQueue.toast.reorderFailedTitle'),
      description: error instanceof Error ? error.message : String(error),
      color: 'error',
    })
  } finally {
    reorderingQueue.value = false
    clearDragState()
  }
}

function handlePointerUp(event: PointerEvent) {
  if (dragPointerId == null || event.pointerId !== dragPointerId) return
  window.removeEventListener('pointermove', handlePointerMove)
  window.removeEventListener('pointerup', handlePointerUp)
  window.removeEventListener('pointercancel', handlePointerCancel)
  if (!pointerDragging.value) {
    clearDragState()
    return
  }
  void commitDrop()
}

function handlePointerCancel() {
  window.removeEventListener('pointermove', handlePointerMove)
  window.removeEventListener('pointerup', handlePointerUp)
  window.removeEventListener('pointercancel', handlePointerCancel)
  clearDragState()
}

function handleCardClick(job: FfmpegJob, event: MouseEvent) {
  const target = event.target as HTMLElement | null
  if (target?.closest('button, a, input, textarea, select, [data-no-drag], [role="menuitem"], [role="checkbox"], [contenteditable="true"]')) {
    return
  }
  if (pointerDragging.value) return

  const now = Date.now()
  if (lastClickJobId === job.id && now - lastClickTime < DBLCLICK_MAX_DELAY) {
    lastClickJobId = null
    lastClickTime = 0
    router.push(`/transcode/${job.id}`)
  } else {
    lastClickJobId = job.id
    lastClickTime = now
  }
}

function handlePointerDown(job: FfmpegJob, event: PointerEvent) {
  if (event.button !== 0) return
  if (!canDragJob(job) || reorderingQueue.value) return
  const target = event.target as HTMLElement | null
  if (target?.closest('button, a, input, textarea, select, [data-no-drag], [role="menuitem"], [role="checkbox"], [contenteditable="true"]')) {
    return
  }

  dragPointerId = event.pointerId
  pendingDragJobId = job.id
  dragStartX = event.clientX
  dragStartY = event.clientY
  window.addEventListener('pointermove', handlePointerMove)
  window.addEventListener('pointerup', handlePointerUp)
  window.addEventListener('pointercancel', handlePointerCancel)
}

async function confirmClearCompletedTranscodeJobs() {
  if (clearingCompletedTranscodeJobs.value || doneJobs.value.length === 0) return

  clearingCompletedTranscodeJobs.value = true
  try {
    const { removed, failed } = await transcodeStore.clearCompletedJobs()
    if (removed > 0) {
      toast.add({
        title: t('transcodeQueue.toast.clearSuccessTitle'),
        description: failed > 0
          ? t('transcodeQueue.toast.clearPartialDescription', { removed, failed })
          : t('transcodeQueue.toast.clearSuccessDescription', { removed }),
        color: failed > 0 ? 'warning' : 'success',
      })
    } else if (failed > 0) {
      toast.add({
        title: t('transcodeQueue.toast.clearFailedTitle'),
        description: t('transcodeQueue.toast.clearFailedDescription', { failed }),
        color: 'error',
      })
    }
  } finally {
    showClearCompletedConfirm.value = false
    clearingCompletedTranscodeJobs.value = false
  }
}

async function handleCancel(id: string) {
  try {
    await transcodeStore.cancelFfmpegJob(id)
  } catch (error) {
    toast.add({
      title: t('transcodeQueue.toast.cancelFailedTitle'),
      description: error instanceof Error ? error.message : String(error),
      color: 'error',
    })
  }
}

async function confirmDelete() {
  if (!deleteConfirmJob.value) return
  try {
    await transcodeStore.deleteFfmpegJob(deleteConfirmJob.value.id)
  } catch (error) {
    toast.add({
      title: t('transcodeQueue.toast.deleteFailedTitle'),
      description: error instanceof Error ? error.message : String(error),
      color: 'error',
    })
  } finally {
    deleteConfirmJob.value = null
  }
}

onMounted(async () => {
  await Promise.all([
    settingsStore.load(),
    transcodeStore.fetchFfmpegJobs(),
  ])

  unlisteners.push(await onTranscodeProgress(event => transcodeStore.applyProgress(event)))
  unlisteners.push(await onTranscodeLog(event => transcodeStore.applyLog(event)))
  unlisteners.push(await onFfmpegJobUpdated(event => transcodeStore.applyFfmpegJobUpdate(event)))

  const unlistenDrop = await getCurrentWindow().onDragDropEvent(async (event) => {
    if (route.path !== '/transcode') return
    if (draggedJobId.value) return
    if (event.payload.type === 'enter' || event.payload.type === 'over') {
      isDragging.value = true
      return
    }
    if (event.payload.type === 'leave') {
      isDragging.value = false
      return
    }
    if (event.payload.type === 'drop') {
      isDragging.value = false
      const paths = Array.from(new Set(event.payload.paths as string[]))
      for (const path of paths) {
        await addFolderToQueue(path)
      }
    }
  })
  unlisteners.push(unlistenDrop)
})

onUnmounted(() => {
  for (const unlisten of unlisteners) {
    unlisten()
  }
  window.removeEventListener('pointermove', handlePointerMove)
  window.removeEventListener('pointerup', handlePointerUp)
  window.removeEventListener('pointercancel', handlePointerCancel)
  document.body.style.removeProperty('user-select')
})
</script>
