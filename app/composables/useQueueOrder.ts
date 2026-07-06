import type { FfmpegJobStatus, JobStatus } from '~/types'

type QueueItem<TStatus extends string = string> = {
  id: string
  status: TStatus
}

export const RENDER_QUEUE_ORDER_HIDDEN_STATUSES: readonly JobStatus[] = ['running', 'done']
export const FFMPEG_QUEUE_ORDER_HIDDEN_STATUSES: readonly FfmpegJobStatus[] = ['running', 'done']

export function resolveQueueOrder<TStatus extends string, TItem extends QueueItem<TStatus>>(
  items: readonly TItem[],
  currentItem: TItem | null | undefined,
  hiddenStatuses: readonly TStatus[],
) {
  if (!currentItem || hiddenStatuses.includes(currentItem.status)) return null

  const queue = items.filter(item => !hiddenStatuses.includes(item.status))
  const index = queue.findIndex(item => item.id === currentItem.id)
  return index === -1 ? null : index + 1
}

export function useQueueOrderLabel() {
  const { t } = useI18n()
  return (order: number | null) => order != null ? t('queue.order', { order }) : null
}
