<script setup lang="ts">
withDefaults(
  defineProps<{
    title: string
    message: string
    confirmLabel?: string
    danger?: boolean
    busy?: boolean
  }>(),
  { confirmLabel: 'Confirm', danger: false, busy: false },
)

const emit = defineEmits<{ (event: 'confirm'): void; (event: 'cancel'): void }>()
</script>

<template>
  <div class="modal-backdrop" @click.self="emit('cancel')">
    <div class="modal" style="width: min(440px, 100%)">
      <div class="modal-head">
        <h2>{{ title }}</h2>
      </div>
      <div class="modal-body">
        <p class="small" style="color: var(--text-2); line-height: 1.6">{{ message }}</p>
      </div>
      <div class="modal-foot">
        <button class="btn" @click="emit('cancel')">Cancel</button>
        <button class="btn" :class="danger ? 'danger' : 'primary'" :disabled="busy" @click="emit('confirm')">
          {{ confirmLabel }}
        </button>
      </div>
    </div>
  </div>
</template>
