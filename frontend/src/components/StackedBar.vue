<script setup lang="ts">
import { computed } from 'vue'

export interface Segment {
  key: string
  label: string
  value: number
  color: string
  text?: string
}

const props = defineProps<{ segments: Segment[]; height?: number }>()

const total = computed(() => props.segments.reduce((sum, segment) => sum + Math.max(0, segment.value), 0))

const visible = computed(() =>
  props.segments
    .filter((segment) => segment.value > 0)
    .map((segment) => ({
      ...segment,
      width: total.value > 0 ? (Math.max(0, segment.value) / total.value) * 100 : 0,
    })),
)
</script>

<template>
  <div>
    <div class="stack" :style="height ? { height: `${height}px` } : undefined">
      <div
        v-for="segment in visible"
        :key="segment.key"
        class="seg"
        :style="{ width: `${segment.width}%`, background: segment.color }"
        :title="`${segment.label}: ${segment.text ?? segment.value}`"
      />
    </div>
    <div v-if="$slots.legend" class="legend"><slot name="legend" /></div>
  </div>
</template>
