import { defineStore } from 'pinia'
import { listen } from '@tauri-apps/api/event'
import type {
  NodeInfo,
  PeerDiscoveredEvent,
  PeerInfo,
  PeerJobUpdatedEvent,
  PeerLostEvent,
  PeerProgressEvent,
  PeerQueueStateEvent,
} from '~/types'

export const useNodesStore = defineStore('nodes', () => {
  const peers = ref<Record<string, PeerInfo>>({})
  const localNode = ref<NodeInfo | null>(null)
  const initialized = ref(false)
  const unlisteners: Array<() => void> = []
  const { getNodeInfo, getPeers } = useTauri()

  async function init() {
    localNode.value = await getNodeInfo()
    const list = await getPeers()
    peers.value = Object.fromEntries(list.map(peer => [peer.node.id, peer]))

    if (initialized.value) return
    initialized.value = true

    unlisteners.push(await listen<PeerDiscoveredEvent>('peer-discovered', ({ payload }) => {
      peers.value[payload.peer.node.id] = payload.peer
    }))

    unlisteners.push(await listen<PeerLostEvent>('peer-lost', ({ payload }) => {
      delete peers.value[payload.nodeId]
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
  }

  function dispose() {
    while (unlisteners.length) {
      unlisteners.pop()?.()
    }
    initialized.value = false
  }

  const peerList = computed(() => Object.values(peers.value))
  const connectedCount = computed(() => peerList.value.filter(peer => peer.connected).length)

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

  return {
    peers,
    peerList,
    localNode,
    connectedCount,
    init,
    dispose,
  }
})
