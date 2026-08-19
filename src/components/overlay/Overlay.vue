<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
import type { ScreenCaptureResult } from '../../stores/screenshot'
import SizeInfo from './SizeInfo.vue'
import ActionBar from './ActionBar.vue'
import Magnifier from './Magnifier.vue'

const appWindow = getCurrentWebviewWindow()

const captureResult = ref<ScreenCaptureResult | null>(null)
const imageSrc = ref('')
const imageLoaded = ref(false)
const selection = ref<{ x: number; y: number; width: number; height: number } | null>(null)

// 交互状态
type DragMode = 'none' | 'create' | 'move' | 'resize-tl' | 'resize-tr' | 'resize-bl' | 'resize-br' | 'resize-t' | 'resize-b' | 'resize-l' | 'resize-r'
const dragMode = ref<DragMode>('none')
const dragStart = ref<{ x: number; y: number } | null>(null)
const selectionAtDragStart = ref<{ x: number; y: number; width: number; height: number } | null>(null)

const HANDLE_SIZE = 6
const MIN_SIZE = 5
const NUDGE_PX = 1 // 方向键微调步长
const NUDGE_PX_LARGE = 10 // Shift+方向键微调步长

// 鼠标位置
const mouseX = ref(-1)
const mouseY = ref(-1)

// 屏幕尺寸
const screenWidth = ref(window.innerWidth)
const screenHeight = ref(window.innerHeight)

// 底图 canvas 缓存
const bgCanvas = ref<HTMLCanvasElement | null>(null)

// 窗口智能识别
interface WindowInfo {
  x: number
  y: number
  width: number
  height: number
  title: string
  app_name: string
}

const windowList = ref<WindowInfo[]>([])
const hoveredWindow = ref<WindowInfo | null>(null)

onMounted(async () => {
  // 主动拉取截图数据
  try {
    const result = await invoke<ScreenCaptureResult>('get_pending_capture')
    captureResult.value = result
    const src = `data:image/png;base64,${result.image_base64}`

    // 预加载图片
    const img = new Image()
    img.onload = async () => {
      imageSrc.value = src
      imageLoaded.value = true

      // 创建底图 Canvas 缓存
      const canvas = document.createElement('canvas')
      canvas.width = img.naturalWidth
      canvas.height = img.naturalHeight
      const ctx = canvas.getContext('2d')
      if (ctx) {
        ctx.drawImage(img, 0, 0)
        bgCanvas.value = canvas
      }

      // 图片就绪后设置焦点
      try {
        await appWindow.setFocus()
      } catch (e) {
        console.error('Failed to set focus on overlay window:', e)
      }
    }
    img.onerror = () => {
      console.error('Failed to load screenshot image')
    }
    img.src = src
  } catch (e) {
    console.error('Failed to get pending capture:', e)
  }

  // 获取窗口列表（用于智能识别）
  try {
    const windows = await invoke<WindowInfo[]>('get_visible_windows')
    windowList.value = windows
  } catch (e) {
    console.error('Failed to get window list:', e)
  }

  document.addEventListener('keydown', handleKeyDown)
  document.body.style.cursor = 'crosshair'
})

onUnmounted(() => {
  document.removeEventListener('keydown', handleKeyDown)
})

function handleKeyDown(e: KeyboardEvent) {
  if (e.key === 'Escape') {
    closeOverlay()
  } else if (e.key === 'Enter' && selection.value) {
    confirmScreenshot()
  } else if (selection.value && ['ArrowUp', 'ArrowDown', 'ArrowLeft', 'ArrowRight'].includes(e.key)) {
    // 选区微调
    e.preventDefault()
    const step = e.shiftKey ? NUDGE_PX_LARGE : NUDGE_PX
    const sel = { ...selection.value }

    if (e.ctrlKey || e.metaKey) {
      // Ctrl/Cmd + 方向键 = 调整大小
      switch (e.key) {
        case 'ArrowRight': sel.width += step; break
        case 'ArrowLeft': sel.width = Math.max(MIN_SIZE, sel.width - step); break
        case 'ArrowDown': sel.height += step; break
        case 'ArrowUp': sel.height = Math.max(MIN_SIZE, sel.height - step); break
      }
    } else {
      // 方向键 = 移动位置
      switch (e.key) {
        case 'ArrowRight': sel.x += step; break
        case 'ArrowLeft': sel.x = Math.max(0, sel.x - step); break
        case 'ArrowDown': sel.y += step; break
        case 'ArrowUp': sel.y = Math.max(0, sel.y - step); break
      }
    }

    selection.value = sel
  }
}

async function closeOverlay() {
  try {
    // 关闭前先显示主窗口
    await invoke('show_main_window')
    await appWindow.destroy()
  } catch (e) {
    console.error('Failed to close overlay:', e)
  }
}

// 构建裁剪参数（逻辑坐标转物理像素坐标）
function buildCropRect(): { x: number; y: number; width: number; height: number } | null {
  if (!selection.value || !captureResult.value) return null
  const sel = selection.value
  const scale = captureResult.value.scale_factor || 1
  return {
    x: Math.round(sel.x * scale),
    y: Math.round(sel.y * scale),
    width: Math.round(sel.width * scale),
    height: Math.round(sel.height * scale),
  }
}

async function confirmScreenshot() {
  if (!selection.value || !captureResult.value) return
  const crop = buildCropRect()
  if (!crop) return

  try {
    await invoke('copy_to_clipboard', {
      imageBase64: captureResult.value.image_base64,
      crop: crop,
    })
  } catch (e) {
    console.error('copy_to_clipboard error:', e)
    alert('复制失败: ' + e)
    return
  }
  await closeOverlay()
}

async function saveScreenshot() {
  if (!selection.value || !captureResult.value) return
  const crop = buildCropRect()
  if (!crop) return

  try {
    await invoke('save_screenshot_with_dialog', {
      imageBase64: captureResult.value.image_base64,
      crop: crop,
    })
  } catch (e) {
    console.error('save_screenshot_with_dialog error:', e)
    // 用户取消不提示
    if (String(e).includes('用户取消了保存')) return
    alert('保存失败: ' + e)
    return
  }
  await closeOverlay()
}

// 检测鼠标是否在调整手柄上
function getHandleAtPoint(x: number, y: number): DragMode | null {
  if (!selection.value) return null

  const sel = selection.value
  const h = HANDLE_SIZE / 2

  if (Math.abs(x - sel.x) < h && Math.abs(y - sel.y) < h) return 'resize-tl'
  if (Math.abs(x - (sel.x + sel.width)) < h && Math.abs(y - sel.y) < h) return 'resize-tr'
  if (Math.abs(x - sel.x) < h && Math.abs(y - (sel.y + sel.height)) < h) return 'resize-bl'
  if (Math.abs(x - (sel.x + sel.width)) < h && Math.abs(y - (sel.y + sel.height)) < h) return 'resize-br'

  if (Math.abs(y - sel.y) < h && x > sel.x + h && x < sel.x + sel.width - h) return 'resize-t'
  if (Math.abs(y - (sel.y + sel.height)) < h && x > sel.x + h && x < sel.x + sel.width - h) return 'resize-b'
  if (Math.abs(x - sel.x) < h && y > sel.y + h && y < sel.y + sel.height - h) return 'resize-l'
  if (Math.abs(x - (sel.x + sel.width)) < h && y > sel.y + h && y < sel.y + sel.height - h) return 'resize-r'

  if (x > sel.x && x < sel.x + sel.width && y > sel.y && y < sel.y + sel.height) return 'move'

  return null
}

function getCursorForMode(mode: DragMode): string {
  switch (mode) {
    case 'resize-tl':
    case 'resize-br': return 'nwse-resize'
    case 'resize-tr':
    case 'resize-bl': return 'nesw-resize'
    case 'resize-t':
    case 'resize-b': return 'ns-resize'
    case 'resize-l':
    case 'resize-r': return 'ew-resize'
    case 'move': return 'move'
    default: return 'crosshair'
  }
}

// 查找鼠标下的窗口（从最小的开始匹配，优先匹配更精确的小窗口）
function findWindowAtPoint(x: number, y: number): WindowInfo | null {
  for (const win of windowList.value) {
    if (x >= win.x && x <= win.x + win.width && y >= win.y && y <= win.y + win.height) {
      return win
    }
  }
  return null
}

function onMouseDown(e: MouseEvent) {
  if (!captureResult.value) return
  if (e.button !== 0) return

  const x = e.clientX
  const y = e.clientY

  // 检测是否在已有选区的手柄/内部
  const handle = getHandleAtPoint(x, y)
  if (handle && selection.value) {
    dragMode.value = handle
    dragStart.value = { x, y }
    selectionAtDragStart.value = { ...selection.value }
    hoveredWindow.value = null
    return
  }

  // 双击选中悬停窗口
  if (e.detail === 2 && hoveredWindow.value) {
    const win = hoveredWindow.value
    selection.value = {
      x: win.x,
      y: win.y,
      width: win.width,
      height: win.height,
    }
    hoveredWindow.value = null
    return
  }

  // 创建新选区
  dragMode.value = 'create'
  dragStart.value = { x, y }
  selection.value = { x, y, width: 0, height: 0 }
  hoveredWindow.value = null
}

function onMouseMove(e: MouseEvent) {
  const x = e.clientX
  const y = e.clientY
  mouseX.value = x
  mouseY.value = y

  // 更新窗口悬停检测（仅在无选区时）
  if (!selection.value && dragMode.value === 'none') {
    hoveredWindow.value = findWindowAtPoint(x, y)
  }

  // 更新光标
  if (dragMode.value === 'none') {
    if (selection.value) {
      const handle = getHandleAtPoint(x, y)
      document.body.style.cursor = handle ? getCursorForMode(handle) : 'crosshair'
    } else {
      // 悬停到窗口上时显示 move 光标
      document.body.style.cursor = hoveredWindow.value ? 'pointer' : 'crosshair'
    }
    return
  }

  if (!dragStart.value) return

  if (dragMode.value === 'create') {
    selection.value = {
      x: Math.min(dragStart.value.x, x),
      y: Math.min(dragStart.value.y, y),
      width: Math.abs(x - dragStart.value.x),
      height: Math.abs(y - dragStart.value.y),
    }
  } else if (dragMode.value === 'move' && selectionAtDragStart.value) {
    const dx = x - dragStart.value.x
    const dy = y - dragStart.value.y
    let newX = selectionAtDragStart.value.x + dx
    let newY = selectionAtDragStart.value.y + dy
    newX = Math.max(0, Math.min(newX, screenWidth.value - selectionAtDragStart.value.width))
    newY = Math.max(0, Math.min(newY, screenHeight.value - selectionAtDragStart.value.height))
    selection.value = {
      x: newX,
      y: newY,
      width: selectionAtDragStart.value.width,
      height: selectionAtDragStart.value.height,
    }
  } else if (dragMode.value.startsWith('resize') && selectionAtDragStart.value) {
    const dx = x - dragStart.value.x
    const dy = y - dragStart.value.y
    const orig = selectionAtDragStart.value

    let newX = orig.x
    let newY = orig.y
    let newW = orig.width
    let newH = orig.height

    if (dragMode.value === 'resize-tl') { newX = orig.x + dx; newY = orig.y + dy; newW = orig.width - dx; newH = orig.height - dy }
    else if (dragMode.value === 'resize-tr') { newY = orig.y + dy; newW = orig.width + dx; newH = orig.height - dy }
    else if (dragMode.value === 'resize-bl') { newX = orig.x + dx; newW = orig.width - dx; newH = orig.height + dy }
    else if (dragMode.value === 'resize-br') { newW = orig.width + dx; newH = orig.height + dy }
    else if (dragMode.value === 'resize-t') { newY = orig.y + dy; newH = orig.height - dy }
    else if (dragMode.value === 'resize-b') { newH = orig.height + dy }
    else if (dragMode.value === 'resize-l') { newX = orig.x + dx; newW = orig.width - dx }
    else if (dragMode.value === 'resize-r') { newW = orig.width + dx }

    if (newW < 0) { newX = newX + newW; newW = -newW }
    if (newH < 0) { newY = newY + newH; newH = -newH }

    if (newW >= MIN_SIZE && newH >= MIN_SIZE) {
      selection.value = { x: newX, y: newY, width: newW, height: newH }
    }
  }
}

function onMouseUp() {
  if (dragMode.value === 'create' && selection.value) {
    if (selection.value.width < MIN_SIZE || selection.value.height < MIN_SIZE) {
      selection.value = null
    }
  }
  dragMode.value = 'none'
  dragStart.value = null
  selectionAtDragStart.value = null
}
</script>

<template>
  <div
    class="fixed inset-0 overflow-hidden select-none"
    @mousedown="onMouseDown"
    @mousemove="onMouseMove"
    @mouseup="onMouseUp"
  >
    <!-- 截图底图 -->
    <img
      v-if="imageSrc"
      id="screenshot-bg"
      :src="imageSrc"
      class="absolute inset-0 w-full h-full object-fill"
      draggable="false"
    />

    <!-- 半透明遮罩 (SVG mask 挖洞) -->
    <svg class="absolute inset-0 w-full h-full pointer-events-none">
      <defs>
        <mask id="overlay-mask">
          <rect x="0" y="0" width="100%" height="100%" fill="white" />
          <!-- 选区挖洞 -->
          <rect
            v-if="selection && selection.width > 0 && selection.height > 0"
            :x="selection.x"
            :y="selection.y"
            :width="selection.width"
            :height="selection.height"
            fill="black"
          />
          <!-- 悬停窗口挖洞 -->
          <rect
            v-else-if="hoveredWindow"
            :x="hoveredWindow.x"
            :y="hoveredWindow.y"
            :width="hoveredWindow.width"
            :height="hoveredWindow.height"
            fill="black"
          />
        </mask>
      </defs>
      <rect
        x="0" y="0" width="100%" height="100%"
        fill="rgba(0,0,0,0.35)"
        mask="url(#overlay-mask)"
      />
    </svg>

    <!-- 悬停窗口高亮边框（无选区时） -->
    <div
      v-if="hoveredWindow && !selection"
      class="absolute pointer-events-none border-2 border-blue-400"
      :style="{
        left: hoveredWindow.x + 'px',
        top: hoveredWindow.y + 'px',
        width: hoveredWindow.width + 'px',
        height: hoveredWindow.height + 'px',
      }"
    >
      <!-- 窗口标题 -->
      <div class="absolute -top-6 left-0 px-1.5 py-0.5 bg-blue-500 text-white text-[10px] rounded-t whitespace-nowrap max-w-[200px] truncate">
        {{ hoveredWindow.app_name || hoveredWindow.title }}
      </div>
    </div>

    <!-- 选区边框 -->
    <div
      v-if="selection && selection.width >= MIN_SIZE && selection.height >= MIN_SIZE"
      class="absolute pointer-events-none border-2 border-blue-500"
      :style="{
        left: selection.x + 'px',
        top: selection.y + 'px',
        width: selection.width + 'px',
        height: selection.height + 'px',
      }"
    >
      <!-- 四角调整手柄 -->
      <div class="absolute -top-[3px] -left-[3px] w-[6px] h-[6px] bg-blue-500 pointer-events-auto" />
      <div class="absolute -top-[3px] -right-[3px] w-[6px] h-[6px] bg-blue-500 pointer-events-auto" />
      <div class="absolute -bottom-[3px] -left-[3px] w-[6px] h-[6px] bg-blue-500 pointer-events-auto" />
      <div class="absolute -bottom-[3px] -right-[3px] w-[6px] h-[6px] bg-blue-500 pointer-events-auto" />
      <!-- 四边中点手柄 -->
      <div class="absolute -top-[3px] left-1/2 -translate-x-1/2 w-[6px] h-[6px] bg-blue-500 pointer-events-auto" />
      <div class="absolute -bottom-[3px] left-1/2 -translate-x-1/2 w-[6px] h-[6px] bg-blue-500 pointer-events-auto" />
      <div class="absolute top-1/2 -translate-y-1/2 -left-[3px] w-[6px] h-[6px] bg-blue-500 pointer-events-auto" />
      <div class="absolute top-1/2 -translate-y-1/2 -right-[3px] w-[6px] h-[6px] bg-blue-500 pointer-events-auto" />
    </div>

    <!-- 尺寸信息 -->
    <SizeInfo v-if="selection && selection.width >= MIN_SIZE && selection.height >= MIN_SIZE" :selection="selection" />

    <!-- 操作栏 -->
    <ActionBar
      v-if="selection && selection.width >= MIN_SIZE && selection.height >= MIN_SIZE && dragMode === 'none'"
      :selection="selection"
      @confirm="confirmScreenshot"
      @save="saveScreenshot"
      @cancel="closeOverlay"
    />

    <!-- 十字准线（未创建选区且无悬停窗口时显示） -->
    <svg
      v-if="!selection && !hoveredWindow && dragMode === 'none' && mouseX >= 0"
      class="absolute inset-0 w-full h-full pointer-events-none z-40"
    >
      <line :x1="mouseX" y1="0" :x2="mouseX" :y2="screenHeight" stroke="rgba(26,115,232,0.5)" stroke-width="1" stroke-dasharray="4,4" />
      <line x1="0" :y1="mouseY" :x2="screenWidth" :y2="mouseY" stroke="rgba(26,115,232,0.5)" stroke-width="1" stroke-dasharray="4,4" />
      <rect :x="mouseX + 8" :y="mouseY + 8" width="90" height="20" rx="3" fill="rgba(0,0,0,0.7)" />
      <text :x="mouseX + 14" :y="mouseY + 22" fill="white" font-size="11" font-family="monospace">
        {{ Math.round(mouseX * (captureResult?.scale_factor || 1)) }}, {{ Math.round(mouseY * (captureResult?.scale_factor || 1)) }}
      </text>
    </svg>

    <!-- 创建选区时的十字准线 -->
    <svg
      v-if="selection && dragMode === 'create' && mouseX >= 0"
      class="absolute inset-0 w-full h-full pointer-events-none z-40"
    >
      <line :x1="mouseX" y1="0" :x2="mouseX" :y2="screenHeight" stroke="rgba(26,115,232,0.3)" stroke-width="1" />
      <line x1="0" :y1="mouseY" :x2="screenWidth" :y2="mouseY" stroke="rgba(26,115,232,0.3)" stroke-width="1" />
    </svg>

    <!-- 放大镜（未创建选区且无悬停窗口时显示） -->
    <Magnifier
      v-if="!selection && !hoveredWindow && dragMode === 'none'"
      :mouse-x="mouseX"
      :mouse-y="mouseY"
      :bg-canvas="bgCanvas"
      :scale-factor="captureResult?.scale_factor || 1"
      :visible="imageLoaded && mouseX >= 0"
    />
  </div>
</template>
