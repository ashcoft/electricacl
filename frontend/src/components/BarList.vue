<script setup lang="ts">
import { computed } from 'vue'
import { fmt } from '../lib/format'

const props = defineProps<{
  rows: { key: string; value: number; note?: string }[]
  unit?: string
  digits?: number
  color?: string
}>()

const max = computed(() => Math.max(1, ...props.rows.map((row) => Math.abs(row.value))))
</script>

<template>
  <div class="barlist">
    <div v-for="row in rows" :key="row.key" class="row">
      <span class="label" :title="row.key">{{ row.key }}</span>
      <div class="track">
        <div
          class="fill"
          :style="{
            width: `${Math.max(1.5, (Math.abs(row.value) / max) * 100)}%`,
            background: color,
          }"
        />
      </div>
      <span class="value">
        {{ fmt(row.value, digits ?? 0) }} <span class="muted" style="font-size: 10px">{{ unit }}</span>
      </span>
    </div>
  </div>
</template>
