export type CapturedVideoPoster = {
  dataUrl: string
  width: number
  height: number
}

export async function captureVideoPoster(url: string) {
  return new Promise<CapturedVideoPoster | null>((resolve) => {
    const video = document.createElement('video')
    let settled = false
    let timeoutId = 0
    let targetTime = 0

    video.preload = 'auto'
    video.muted = true
    video.playsInline = true
    video.crossOrigin = 'anonymous'
    video.src = url

    const cleanup = () => {
      window.clearTimeout(timeoutId)
      video.pause()
      video.removeAttribute('src')
      video.load()
    }

    const succeed = (payload: CapturedVideoPoster) => {
      if (settled) return
      settled = true
      cleanup()
      resolve(payload)
    }

    const fail = () => {
      if (settled) return
      settled = true
      cleanup()
      resolve(null)
    }

    const captureFrame = () => {
      try {
        const width = video.videoWidth
        const height = video.videoHeight
        if (!width || !height || video.readyState < HTMLMediaElement.HAVE_CURRENT_DATA) {
          return false
        }

        const canvas = document.createElement('canvas')
        canvas.width = width
        canvas.height = height
        const context = canvas.getContext('2d')
        if (!context) {
          fail()
          return true
        }

        context.drawImage(video, 0, 0, width, height)
        const dataUrl = canvas.toDataURL('image/jpeg', 0.88)
        succeed({ dataUrl, width, height })
        return true
      } catch {
        fail()
        return true
      }
    }

    const queueCapture = () => {
      const requestFrame = (video as HTMLVideoElement & {
        requestVideoFrameCallback?: (callback: () => void) => number
      }).requestVideoFrameCallback

      if (requestFrame) {
        requestFrame.call(video, () => {
          captureFrame()
        })
        return
      }

      queueMicrotask(() => {
        captureFrame()
      })
    }

    video.onerror = fail
    video.onloadedmetadata = () => {
      if (!Number.isFinite(video.duration) || video.duration <= 0) {
        targetTime = 0
        queueCapture()
        return
      }

      targetTime = Math.max(video.duration - 0.05, 0)
      if (Math.abs(video.currentTime - targetTime) < 0.001) {
        queueCapture()
        return
      }

      video.currentTime = targetTime
    }

    video.onloadeddata = () => {
      if (Math.abs(video.currentTime - targetTime) < 0.05) {
        queueCapture()
      }
    }

    video.oncanplay = () => {
      if (Math.abs(video.currentTime - targetTime) < 0.05) {
        queueCapture()
      }
    }

    video.onseeked = () => {
      queueCapture()
    }

    timeoutId = window.setTimeout(() => {
      if (!captureFrame()) {
        fail()
      }
    }, 2500)
  })
}
