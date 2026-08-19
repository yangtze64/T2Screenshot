<script setup lang="ts">
import { computed } from 'vue'

const props = defineProps<{
  selection: { x: number; y: number; width: number; height: number }
}>()

const displayText = computed(() => {
  return `${Math.round(props.selection.width)} × ${Math.round(props.selection.height)}`
})
</script>

<template>
  <div
    class="absolute pointer-events-none select-none"
    :style="{
      left: selection.x + 'px',
      top: Math.max(0, selection.y - 28) + 'px',
    }"
  >
    <div
      class="bg-black/75 text-white text-xs px-2 py-1 rounded whitespace-nowrap"
    >
      {{ displayText }}
    </div>
  </div>
</template>
