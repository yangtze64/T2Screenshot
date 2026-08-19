<script setup lang="ts">
import { computed } from 'vue'

const props = defineProps<{
  selection: { x: number; y: number; width: number; height: number }
}>()

const emit = defineEmits<{
  confirm: []
  save: []
  cancel: []
}>()

const BAR_WIDTH = 220
const BAR_HEIGHT = 40

const barStyle = computed(() => {
  const sel = props.selection
  const screenW = window.innerWidth
  const screenH = window.innerHeight

  // 默认放在选区右下角下方
  let left = sel.x + sel.width - BAR_WIDTH
  let top = sel.y + sel.height + 8

  // 如果右边超出屏幕，向左移动
  if (left + BAR_WIDTH > screenW) {
    left = screenW - BAR_WIDTH - 4
  }
  // 如果左边超出屏幕
  if (left < 4) {
    left = 4
  }
  // 如果下方超出屏幕，放到选区上方
  if (top + BAR_HEIGHT > screenH) {
    top = sel.y - BAR_HEIGHT - 8
  }
  // 如果上方也超出，放到选区内部底部
  if (top < 4) {
    top = sel.y + sel.height - BAR_HEIGHT - 8
  }

  return { left: left + 'px', top: top + 'px' }
})
</script>

<template>
  <div
    class="absolute flex gap-2 pointer-events-auto"
    :style="barStyle"
    @mousedown.stop
    @mouseup.stop
    @click.stop
  >
    <button
      class="px-3 py-1.5 bg-blue-500 hover:bg-blue-600 text-white text-sm rounded transition-colors"
      @click="emit('confirm')"
    >
      复制
    </button>
    <button
      class="px-3 py-1.5 bg-green-500 hover:bg-green-600 text-white text-sm rounded transition-colors"
      @click="emit('save')"
    >
      保存
    </button>
    <button
      class="px-3 py-1.5 bg-gray-500 hover:bg-gray-600 text-white text-sm rounded transition-colors"
      @click="emit('cancel')"
    >
      取消
    </button>
  </div>
</template>
