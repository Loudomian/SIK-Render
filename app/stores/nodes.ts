import { defineStore } from 'pinia'
import { listen } from '@tauri-apps/api/event'
import type {
  NodeJobEvent,
  NodeInfo,
  PeerDiscoveredEvent,
  PeerInfo,
  PeerJobEventPayload,
  PeerJobUpdatedEvent,
  PeerLogEvent,
  PeerLostEvent,
  PeerProgressEvent,
  PeerQueueStateEvent,
} from '~/types'

export const useNodesStore = defineStore('nodes', () => {
  const peers = ref<Record<string, PeerInfo>>({})
  const jobEvents = ref<Record<string, NodeJobEvent[]>>({})
  const peerLogs = ref<Record<string, string[]>>({})
  const localNode = ref<NodeInfo | null>(null)
  const initialized = ref(false)
  const unlisteners: Array<() => void> = []
  const { forgetPeer, getNodeInfo, getNodeJobEvents, getPeers } = useTauri()
  const toast = useToast()
  const { t } = useI18n()

  async function init() {
    localNode.value = await getNodeInfo()
    const list = await getPeers()
    peers.value = mergePeerList(peers.value, list)

    if (initialized.value) return
    initialized.value = true

    unlisteners.push(await listen<PeerDiscoveredEvent>('peer-discovered', ({ payload }) => {
      peers.value[payload.peer.node.id] = payload.peer
    }))

    unlisteners.push(await listen<PeerLostEvent>('peer-lost', ({ payload }) => {
      const peer = peers.value[payload.nodeId]
      if (!peer) return
      peer.connected = false
      peer.lastSeenAt = Date.now()
      peer.lastDisconnectedAt = peer.lastSeenAt
    }))

    unlisteners.push(await listen<PeerJobUpdatedEvent>('peer-job-updated', ({ payload }) => {
      const peer = peers.value[payload.nodeId]
      if (!peer) return
      const index = peer.jobs.findIndex(job => job.id === payload.job.id)
      if (index === -1) {
        peer.jobs.push(payload.job)
      } else {
        const current = peer.jobs[index]
        if (current) {
          peer.jobs[index] = mergePeerJobSnapshot(current, payload.job)
        }
      }
    }))

    unlisteners.push(await listen<PeerQueueStateEvent>('peer-queue-state', ({ payload }) => {
      const peer = peers.value[payload.nodeId]
      if (peer) peer.queuePaused = payload.paused
    }))

    unlisteners.push(await listen<PeerProgressEvent>('peer-progress', ({ payload }) => {
      const peer = peers.value[payload.nodeId]
      if (!peer) return
      const job = peer.jobs.find(item => item.id === payload.jobId)
      if (!job) return
      job.status = 'running'
      job.currentFrame = Math.max(job.currentFrame ?? 0, payload.frame)
      job.totalFrames = payload.totalFrames
      if (payload.timeElapsed > 0) {
        job.timeElapsed = payload.timeElapsed
      }
      if (payload.remainingSecs != null && payload.remainingSecs > 0) {
        job.remainingSecs = payload.remainingSecs
      } else if (payload.frame >= payload.totalFrames) {
        job.remainingSecs = 0
      }
    }))

    unlisteners.push(await listen<PeerJobEventPayload>('peer-job-event', ({ payload }) => {
      pushJobEvent(payload.event)
    }))

    unlisteners.push(await listen<PeerLogEvent>('peer-log', ({ payload }) => {
      pushPeerLog(payload)
    }))
  }

  function dispose() {
    while (unlisteners.length) {
      unlisteners.pop()?.()
    }
    initialized.value = false
    peerLogs.value = {}
  }

  const peerList = computed(() => Object.values(peers.value))
  const connectedCount = computed(() => peerList.value.filter(peer => peer.connected).length)

  function mergePeerList(current: Record<string, PeerInfo>, incoming: PeerInfo[]) {
    const next = { ...current }
    for (const peer of incoming) {
      const existing = next[peer.node.id]
      if (!existing) {
        next[peer.node.id] = peer
        continue
      }

      const jobs = peer.jobs.map((incomingJob) => {
        const existingJob = existing.jobs.find(job => job.id === incomingJob.id)
        return existingJob ? mergePeerJobSnapshot(existingJob, incomingJob) : incomingJob
      })
      for (const existingJob of existing.jobs) {
        if (!jobs.some(job => job.id === existingJob.id)) {
          jobs.push(existingJob)
        }
      }

      next[peer.node.id] = {
        node: peer.node,
        queuePaused: peer.queuePaused,
        connected: peer.connected,
        firstSeenAt: peer.firstSeenAt ?? existing.firstSeenAt ?? null,
        lastSeenAt: peer.lastSeenAt ?? existing.lastSeenAt ?? null,
        lastConnectedAt: peer.lastConnectedAt ?? existing.lastConnectedAt ?? null,
        lastDisconnectedAt: peer.lastDisconnectedAt ?? existing.lastDisconnectedAt ?? null,
        jobs,
      }
    }
    return next
  }

  function jobEventKey(nodeId: string, jobId: string) {
    return `${nodeId}:${jobId}`
  }

  function peerLogKey(nodeId: string, jobId: string) {
    return `${nodeId}:${jobId}`
  }

  function pushJobEvent(event: NodeJobEvent) {
    const key = jobEventKey(event.nodeId, event.jobId)
    const existing = jobEvents.value[key] ?? []
    const index = existing.findIndex(item => item.id === event.id)
    const next = index === -1
      ? [...existing, event]
      : existing.map(item => item.id === event.id ? event : item)
    jobEvents.value[key] = next.sort((a, b) => a.timestamp - b.timestamp).slice(-200)
  }

  function pushPeerLog(event: PeerLogEvent) {
    const key = peerLogKey(event.nodeId, event.jobId)
    const existing = peerLogs.value[key] ?? []
    peerLogs.value[key] = existing.length >= 2000
      ? [...existing.slice(-1999), event.line]
      : [...existing, event.line]
  }

  function mergePeerJobSnapshot(current: PeerInfo['jobs'][number], incoming: PeerInfo['jobs'][number]) {
    const isRunning = incoming.status === 'running'
    const storedFrame = current.currentFrame ?? 0
    const incomingFrame = incoming.currentFrame ?? 0
    const timingIsStale = isRunning && incomingFrame < storedFrame

    return {
      ...current,
      ...incoming,
      currentFrame: isRunning ? (Math.max(storedFrame, incomingFrame) || undefined) : incoming.currentFrame,
      totalFrames: incoming.totalFrames ?? (isRunning ? current.totalFrames : undefined),
      lastRenderedFrame: isRunning
        ? (current.lastRenderedFrame != null && incoming.lastRenderedFrame != null
            ? Math.max(current.lastRenderedFrame, incoming.lastRenderedFrame)
            : (current.lastRenderedFrame ?? incoming.lastRenderedFrame))
        : incoming.lastRenderedFrame,
      timeElapsed: timingIsStale || ((incoming.timeElapsed ?? 0) <= 0 && (current.timeElapsed ?? 0) > 0)
        ? current.timeElapsed
        : incoming.timeElapsed,
      remainingSecs: timingIsStale || (incoming.remainingSecs == null && current.remainingSecs != null)
        ? current.remainingSecs
        : incoming.remainingSecs,
    }
  }

  function getJobEvents(nodeId: string, jobId: string): NodeJobEvent[] {
    return jobEvents.value[jobEventKey(nodeId, jobId)] ?? []
  }

  function getPeerLogs(nodeId: string, jobId: string): string[] {
    return peerLogs.value[peerLogKey(nodeId, jobId)] ?? []
  }

  async function loadJobEvents(nodeId: string, jobId: string) {
    const events = await getNodeJobEvents(nodeId, jobId)
    jobEvents.value[jobEventKey(nodeId, jobId)] = events.slice(-200)
  }

  async function forgetNode(nodeId: string) {
    try {
      await forgetPeer(nodeId)
    } catch (error) {
      toast.add({
        title: t('nodeToasts.forgetFailed'),
        description: errorMessage(error),
        color: 'error',
        icon: 'i-lucide-circle-alert',
      })
      throw error
    }

    const nextPeers = { ...peers.value }
    delete nextPeers[nodeId]
    peers.value = nextPeers

    const nextEvents = { ...jobEvents.value }
    for (const key of Object.keys(nextEvents)) {
      if (key.startsWith(`${nodeId}:`)) {
        delete nextEvents[key]
      }
    }
    jobEvents.value = nextEvents
  }

  function errorMessage(error: unknown) {
    return error instanceof Error ? error.message : String(error)
  }

  return {
    peers,
    jobEvents,
    peerLogs,
    peerList,
    localNode,
    connectedCount,
    forgetNode,
    getJobEvents,
    getPeerLogs,
    loadJobEvents,
    init,
    dispose,
  }
})
