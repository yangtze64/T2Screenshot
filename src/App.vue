<script setup lang="ts">
import { computed, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import Overlay from './components/overlay/Overlay.vue'

const isOverlay = computed(() => {
  return window.location.search.includes('overlay')
})

const statusMsg = ref('')

async function triggerScreenshot() {
  statusMsg.value = ''
  try {
    await invoke('trigger_screenshot')
  } catch (e) {
    statusMsg.value = '截图失败: ' + e
    console.error('trigger_screenshot failed:', e)
  }
}
</script>

<template>
  <Overlay v-if="isOverlay" />
  <div v-else class="flex flex-col items-center justify-center w-full h-screen bg-gray-900 gap-4">
    <h1 class="text-white text-2xl mb-2">T2Screenshot</h1>
    <div class="flex gap-3">
      <button
        class="px-4 py-2 bg-blue-500 hover:bg-blue-600 text-white rounded-lg transition-colors"
        @click="triggerScreenshot"
      >
        截图
      </button>
    </div>
    <p class="text-gray-500 text-sm">或按 Cmd+Shift+A 触发截图</p>
    <p v-if="statusMsg" class="text-yellow-400 text-sm mt-2 max-w-md text-center break-all">{{ statusMsg }}</p>
  </div>
</template>
