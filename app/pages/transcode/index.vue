<template>
  <div class="queue-page">
    <Transition name="drop-fade">
      <div v-if="isDragging && !draggedJobId" class="drop-overlay">
        <div class="drop-message">
          <UIcon name="i-lucide-folder-down" class="drop-icon" />
          <div class="drop-copy">
            <strong>拖拽序列帧文件夹到窗口</strong>
            <span>松开以创建转码任务</span>
          </div>
        </div>
      </div>
    </Transition>

    <section class="queue-header">
      <section class="page-hero queue-hero">
        <div class="page-hero-copy">
          <div class="queue-heading-row">
            <div class="queue-heading-copy">
              <h1>转码队列</h1>
              <p class="page-note">集中查看所有转码任务的状态、结果与日志。</p>
            </div>
            <div class="queue-hero-actions-stack">
              <div class="page-hero-actions queue-hero-actions">
                <UFieldGroup size="md">
                  <UButton
                    icon="i-lucide-plus"
                    label="新建任务"
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
      <div v-if="transcodeStore.loading" class="loading">加载中…</div>

      <UCard
        v-else-if="transcodeStore.ffmpegJobs.length === 0"
        variant="subtle"
        class="empty-state"
        :ui="{ body: 'empty-state-body' }"
      >
        <div class="empty-state-icon">
          <UIcon name="i-lucide-clapperboard" />
        </div>
        <div class="empty-state-title">还没有 FFmpeg Job</div>
        <div class="empty-state-note">拖拽序列帧文件夹到窗口，或点击“新建任务”手动创建转码任务。</div>
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
      title="删除转码任务"
      :ui="{ content: 'job-modal-content' }"
      @update:open="v => { if (!v) deleteConfirmJob = null }"
    >
      <template #body>
        <div class="modal-stack">
          <p class="modal-copy">
            确定删除 <strong>{{ deleteConfirmJob?.name }}</strong>？此操作不可撤销。
          </p>
          <div class="modal-actions">
            <UButton icon="i-lucide-x" label="取消" color="warning" variant="outline" @click="deleteConfirmJob = null" />
            <UButton icon="i-lucide-trash-2" label="删除" color="error" variant="outline" @click="confirmDelete" />
          </div>
        </div>
      </template>
    </UModal>

    <UModal
      :open="showClearCompletedConfirm"
      :close="false"
      title="清理已完成任务"
      :ui="{ content: 'job-modal-content' }"
      @update:open="v => { if (!v) showClearCompletedConfirm = false }"
    >
      <template #body>
        <div class="modal-stack">
          <p class="modal-copy">
            确定清理当前所有
            <strong>{{ doneJobs.length }}</strong>
            个已完成转码任务？此操作不可撤销。
          </p>
          <div class="modal-actions">
            <UButton
              icon="i-lucide-x"
              label="取消"
              color="warning"
              variant="outline"
              :disabled="clearingCompletedTranscodeJobs"
              @click="showClearCompletedConfirm = false"
            />
            <UButton
              icon="i-lucide-trash-2"
              label="确认清理"
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
      return '当前没有排队中任务'
    case 'done':
      return '当前没有已完成任务'
    case 'error':
      return '当前没有已中止任务'
    default:
      return '当前没有任务'
  }
})
const emptyTabNote = computed(() => {
  switch (activeTab.value) {
    case 'queue':
      return '等待中的任务会保留在这里，运行中的任务也会一起显示。'
    case 'done':
      return '完成的 FFmpeg Job 会集中展示，方便回看输出结果。'
    case 'error':
      return '失败和已取消的 FFmpeg Job 会保留在这里，便于重查日志。'
    default:
      return '这里会显示当前筛选下的任务卡片。'
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
    label: '清理已完成',
    icon: 'i-lucide-trash-2',
    disabled: clearingCompletedTranscodeJobs.value || doneJobs.value.length === 0,
    onSelect: () => {
      showClearCompletedConfirm.value = true
    },
  },
]])
const tabItems = computed<TabsItem[]>(() => [
  {
    label: '全部',
    value: 'all',
    badge: { label: String(transcodeStore.ffmpegJobs.length), color: 'neutral' as const, variant: 'subtle' as const },
    icon: 'i-lucide-layers',
    class: 'queue-tab-tone-all',
    ui: { trigger: 'queue-tab-tone-all' },
  },
  {
    label: '排队中',
    value: 'queue',
    badge: { label: String(queueJobs.value.length), color: 'info' as const, variant: 'subtle' as const },
    icon: 'i-lucide-loader-circle',
    class: 'queue-tab-tone-queue',
    ui: { trigger: 'queue-tab-tone-queue' },
  },
  {
    label: '已完成',
    value: 'done',
    badge: { label: String(doneJobs.value.length), color: 'success' as const, variant: 'subtle' as const },
    icon: 'i-lucide-circle-check-big',
    class: 'queue-tab-tone-done',
    ui: { trigger: 'queue-tab-tone-done' },
  },
  {
    label: '已中止',
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
        title: '未发现序列帧',
        description: '这个目录里没有检测到可转码的序列帧。',
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
      title: '创建 FFmpeg Job 失败',
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
      title: '已创建 FFmpeg Job',
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
      title: '创建 FFmpeg Job 失败',
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
      title: '顺序更新失败',
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
        title: '已清理完成任务',
        description: failed > 0 ? `成功清理 ${removed} 个，另有 ${failed} 个失败。` : `成功清理 ${removed} 个已完成任务。`,
        color: failed > 0 ? 'warning' : 'success',
      })
    } else if (failed > 0) {
      toast.add({
        title: '清理完成任务失败',
        description: `共有 ${failed} 个任务未能删除。`,
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
      title: '取消 FFmpeg Job 失败',
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
      title: '删除 FFmpeg Job 失败',
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
