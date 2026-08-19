import { defineStore } from 'pinia'
import { ref } from 'vue'

export interface ScreenCaptureResult {
  image_base64: string
  width: number
  height: number
  full_width: number
  full_height: number
  scale_factor: number
}

export interface SelectionRect {
  x: number
  y: number
  width: number
  height: number
}

export const useScreenshotStore = defineStore('screenshot', () => {
  const captureResult = ref<ScreenCaptureResult | null>(null)
  const selection = ref<SelectionRect | null>(null)
  const isSelecting = ref(false)
  const isOverlayVisible = ref(false)

  function setCaptureResult(result: ScreenCaptureResult) {
    captureResult.value = result
    isOverlayVisible.value = true
  }

  function setSelection(rect: SelectionRect) {
    selection.value = rect
  }

  function startSelection() {
    isSelecting.value = true
    selection.value = null
  }

  function endSelection() {
    isSelecting.value = false
  }

  function clearSelection() {
    selection.value = null
  }

  function hideOverlay() {
    isOverlayVisible.value = false
    captureResult.value = null
    selection.value = null
    isSelecting.value = false
  }

  return {
    captureResult,
    selection,
    isSelecting,
    isOverlayVisible,
    setCaptureResult,
    setSelection,
    startSelection,
    endSelection,
    clearSelection,
    hideOverlay,
  }
})
