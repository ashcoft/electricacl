<script setup lang="ts">
import { fmt } from '../lib/format'

withDefaults(
  defineProps<{
    label: string
    value: number
    unit: string
    accent?: 'cyan' | 'amber' | 'violet' | 'green'
    note?: string
    secondary?: string
  }>(),
  { accent: 'cyan', note: '', secondary: '' },
)
</script>

<template>
  <div class="card kpi" :class="accent !== 'cyan' ? `accent-${accent}` : ''">
    <div class="kpi-label">{{ label }}</div>
    <div class="kpi-value">
      <b>{{ fmt(value, value >= 1000 ? 0 : 1) }}</b>
      <span>{{ unit }}</span>
    </div>
    <div v-if="note || secondary" class="kpi-foot">
      <span v-if="secondary" class="mono">{{ secondary }}</span>
      <span v-if="note">{{ note }}</span>
    </div>
  </div>
</template>
