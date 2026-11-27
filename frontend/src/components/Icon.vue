<script setup lang="ts">
/**
 * Inline SVG icons.
 *
 * The UI used to rely on glyphs such as ⏻, ⚙ and ⌕. Those live in fonts that
 * are not present on every system, so they render as empty boxes for some
 * users. Inline paths always render.
 */
withDefaults(
  defineProps<{
    name:
      | 'power'
      | 'settings'
      | 'download'
      | 'plus'
      | 'dashboard'
      | 'list'
      | 'motor'
      | 'formula'
      | 'search'
      | 'close'
      | 'chevron-left'
      | 'chevron-right'
    size?: number
  }>(),
  { size: 16 },
)
</script>

<template>
  <svg
    :width="size"
    :height="size"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width="1.8"
    stroke-linecap="round"
    stroke-linejoin="round"
    aria-hidden="true"
    focusable="false"
    style="display: block; flex: none"
  >
    <template v-if="name === 'power'">
      <path d="M12 3v9" />
      <path d="M6.4 6.4a8 8 0 1 0 11.2 0" />
    </template>

    <template v-else-if="name === 'settings'">
      <circle cx="12" cy="12" r="3" />
      <path
        d="M12 2.5v2.2M12 19.3v2.2M4.2 7l1.9 1.1M17.9 15.9l1.9 1.1M2.5 12h2.2M19.3 12h2.2M4.2 17l1.9-1.1M17.9 8.1l1.9-1.1"
      />
    </template>

    <template v-else-if="name === 'download'">
      <path d="M12 3v11" />
      <path d="M7.5 10.5 12 15l4.5-4.5" />
      <path d="M4 18.5v1.2A1.3 1.3 0 0 0 5.3 21h13.4a1.3 1.3 0 0 0 1.3-1.3v-1.2" />
    </template>

    <template v-else-if="name === 'plus'">
      <path d="M12 5v14M5 12h14" />
    </template>

    <template v-else-if="name === 'dashboard'">
      <rect x="3" y="3" width="7.5" height="7.5" rx="1.4" />
      <rect x="13.5" y="3" width="7.5" height="7.5" rx="1.4" />
      <rect x="3" y="13.5" width="7.5" height="7.5" rx="1.4" />
      <rect x="13.5" y="13.5" width="7.5" height="7.5" rx="1.4" />
    </template>

    <template v-else-if="name === 'list'">
      <path d="M8 6h13M8 12h13M8 18h13" />
      <path d="M3.5 6h.01M3.5 12h.01M3.5 18h.01" />
    </template>

    <template v-else-if="name === 'motor'">
      <rect x="3" y="7" width="13" height="10" rx="2" />
      <path d="M16 10.5h3.5v3H16M6.5 17v2.5M12.5 17v2.5M6.5 4.5V7M12.5 4.5V7" />
    </template>

    <template v-else-if="name === 'formula'">
      <path d="M6 20c2.5 0 3-1.5 3.6-4.4l1.9-8.4C12 4.2 12.6 3 14.5 3H17" />
      <path d="M8.5 9.5h7" />
    </template>

    <template v-else-if="name === 'search'">
      <circle cx="11" cy="11" r="6.5" />
      <path d="m16 16 4.5 4.5" />
    </template>

    <template v-else-if="name === 'close'">
      <path d="M6 6l12 12M18 6 6 18" />
    </template>

    <template v-else-if="name === 'chevron-left'">
      <path d="m14.5 5-7 7 7 7" />
    </template>

    <template v-else-if="name === 'chevron-right'">
      <path d="m9.5 5 7 7-7 7" />
    </template>
  </svg>
</template>
