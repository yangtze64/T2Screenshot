<script setup lang="ts">
import { ref, watch, onMounted, onUnmounted } from 'vue'

const props = defineProps<{
  mouseX: number
  mouseY: number
  bgCanvas: HTMLCanvasElement | null
  scaleFactor: number
  visible: boolean
}>()

const MAGNIFIER_SIZE = 120
const ZOOM = 3
const PIXEL_COUNT = Math.floor(MAGNIFIER_SIZE / ZOOM)

const canvasRef = ref<HTMLCanvasElement | null>(null)
const colorHex = ref('')
const colorRgb = ref('')

let rafId = 0

function drawMagnifier() {
  if (!props.visible || !props.bgCanvas || !canvasRef.value || props.mouseX < 0) return

  const canvas = canvasRef.value
  const ctx = canvas.getContext('2d')
  if (!ctx) return

  const scale = props.scaleFactor || 1

  // 鼠标位置在物理像素中的坐标
  const physX = Math.round(props.mouseX * scale)
  const physY = Math.round(props.mouseY * scale)

  // 清空
  ctx.clearRect(0, 0, MAGNIFIER_SIZE, MAGNIFIER_SIZE)

  // 从底图 canvas 取像素区域放大绘制
  const srcSize = PIXEL_COUNT
  const srcX = physX - Math.floor(srcSize / 2)
  const srcY = physY - Math.floor(srcSize / 2)

  // 绘制放大区域
  ctx.imageSmoothingEnabled = false
  ctx.drawImage(
    props.bgCanvas,
    srcX, srcY, srcSize, srcSize,
    0, 0, MAGNIFIER_SIZE, MAGNIFIER_SIZE,
  )

  // 绘制网格线
  ctx.strokeStyle = 'rgba(255,255,255,0.15)'
  ctx.lineWidth = 0.5
  for (let i = 0; i <= PIXEL_COUNT; i++) {
    const pos = i * ZOOM
    ctx.beginPath()
    ctx.moveTo(pos, 0)
    ctx.lineTo(pos, MAGNIFIER_SIZE)
    ctx.stroke()
    ctx.beginPath()
    ctx.moveTo(0, pos)
    ctx.lineTo(MAGNIFIER_SIZE, pos)
    ctx.stroke()
  }

  // 中心十字
  const center = MAGNIFIER_SIZE / 2
  ctx.strokeStyle = 'rgba(255,0,0,0.8)'
  ctx.lineWidth = 1
  ctx.beginPath()
  ctx.moveTo(center - ZOOM, center)
  ctx.lineTo(center + ZOOM, center)
  ctx.stroke()
  ctx.beginPath()
  ctx.moveTo(center, center - ZOOM)
  ctx.lineTo(center, center + ZOOM)
  ctx.stroke()

  // 获取中心像素颜色
  try {
    const pixel = props.bgCanvas.getContext('2d')?.getImageData(physX, physY, 1, 1).data
    if (pixel) {
      const r = pixel[0]
      const g = pixel[1]
      const b = pixel[2]
      colorHex.value = `#${r.toString(16).padStart(2, '0')}${g.toString(16).padStart(2, '0')}${b.toString(16).padStart(2, '0')}`
      colorRgb.value = `RGB(${r}, ${g}, ${b})`
    }
  } catch {
    // 跨域或越界
  }
}

// 计算放大镜位置（跟随鼠标，偏移显示）
function getMagnifierStyle() {
  if (!props.visible || props.mouseX < 0) return { display: 'none' }

  const offset = 20
  let left = props.mouseX + offset
  let top = props.mouseY - MAGNIFIER_SIZE - offset

  // 防止超出屏幕
  if (left + MAGNIFIER_SIZE > window.innerWidth) {
    left = props.mouseX - MAGNIFIER_SIZE - offset
  }
  if (top < 0) {
    top = props.mouseY + offset
  }

  return {
    left: `${left}px`,
    top: `${top}px`,
  }
}

watch(() => [props.mouseX, props.mouseY, props.visible, props.bgCanvas], () => {
  cancelAnimationFrame(rafId)
  rafId = requestAnimationFrame(drawMagnifier)
})

onMounted(() => drawMagnifier())
onUnmounted(() => cancelAnimationFrame(rafId))
</script>

<template>
  <div
    v-if="visible && mouseX >= 0"
    class="absolute pointer-events-none z-50"
    :style="getMagnifierStyle()"
  >
    <!-- 放大镜 Canvas -->
    <canvas
      ref="canvasRef"
      :width="MAGNIFIER_SIZE"
      :height="MAGNIFIER_SIZE"
      class="rounded-lg border-2 border-white/60 shadow-xl"
    />
    <!-- 像素信息 -->
    <div class="mt-1 px-1.5 py-0.5 rounded bg-black/70 text-white text-[10px] font-mono whitespace-nowrap flex items-center gap-1.5">
      <span
        class="inline-block w-3 h-3 rounded-sm border border-white/40"
        :style="{ backgroundColor: colorHex }"
      />
      <span>{{ colorHex }}</span>
      <span class="text-white/60">{{ colorRgb }}</span>
    </div>
  </div>
</template>
