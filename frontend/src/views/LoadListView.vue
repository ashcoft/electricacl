<script setup lang="ts">
import { computed, nextTick, reactive, ref, watch } from 'vue'
import ConfirmDialog from '../components/ConfirmDialog.vue'
import { fmt, fmtPct, fmtSmart } from '../lib/format'
import {
  currentSummary,
  data,
  matchesSearch,
  resetFilters,
  resetItems,
  saveItem,
  ui,
} from '../store'
import { CLASSES, DUTIES, SCENARIOS, dutyLabel, type DutyKey, type ItemView } from '../types'

const editing = reactive<{ id: string; field: 'absorbed_kw' | 'rated_kw' | null }>({
  id: '',
  field: null,
})
const editValue = ref('')
const confirmReset = ref(false)
const busyEdit = ref(false)
const sortKey = ref<'source' | 'line_no' | 'tag' | 'consumed' | 'flc'>('source')
const sortDir = ref<'asc' | 'desc'>('asc')

const state = computed(() => data.state)
const scenario = computed(() => ui.scenario)
const summary = currentSummary

const filtered = computed<ItemView[]>(() => {
  const items = state.value?.items ?? []
  const filters = ui.filters
  return items.filter((item) => {
    if (!matchesSearch(item, filters.search)) return false
    if (filters.area && item.area !== filters.area) return false
    if (filters.voltage && Math.abs(item.voltage_kv - Number(filters.voltage)) > 1e-9) return false
    if (filters.duty && item.duty !== filters.duty) return false
    if (filters.klass && item.equipment_class !== filters.klass) return false
    if (filters.status && item.status !== filters.status) return false
    if (filters.warningsOnly && item.derived.warnings.length === 0) return false
    return true
  })
})

const sorted = computed(() => {
  const list = [...filtered.value]
  if (sortKey.value === 'source') return list
  const direction = sortDir.value === 'asc' ? 1 : -1
  list.sort((a, b) => {
    switch (sortKey.value) {
      case 'tag':
        return (a.tag ?? '').localeCompare(b.tag ?? '') * direction
      case 'consumed':
        return (a.derived.consumed_kw - b.derived.consumed_kw) * direction
      case 'flc':
        return (a.derived.flc_a - b.derived.flc_a) * direction
      default:
        return ((a.line_no ?? 0) - (b.line_no ?? 0)) * direction
    }
  })
  return list
})

const pageCount = computed(() => Math.max(1, Math.ceil(sorted.value.length / ui.pageSize)))
const paged = computed(() => {
  const start = (ui.page - 1) * ui.pageSize
  return sorted.value.slice(start, start + ui.pageSize)
})

const rangeLabel = computed(() => {
  if (!sorted.value.length) return '0 items'
  const start = (ui.page - 1) * ui.pageSize + 1
  const end = Math.min(sorted.value.length, ui.page * ui.pageSize)
  return `${start}–${end} of ${sorted.value.length}`
})

const activeInScenario = computed(
  () => filtered.value.filter((item) => item.scenarios.includes(scenario.value)),
)

const footerTotals = computed(() => {
  const totals = { continuous: 0, intermittent: 0, standby: 0, spare: 0, consumed: 0 }
  for (const item of activeInScenario.value) {
    totals.consumed += item.derived.consumed_kw
    if (item.duty) totals[item.duty] += item.derived.consumed_kw
  }
  return totals
})

watch(
  () => [ui.filters, ui.pageSize],
  () => {
    ui.page = 1
  },
  { deep: true },
)

watch(pageCount, (count) => {
  if (ui.page > count) ui.page = count
})

function toggleSort(key: typeof sortKey.value) {
  if (sortKey.value === key) {
    sortDir.value = sortDir.value === 'asc' ? 'desc' : 'asc'
  } else {
    sortKey.value = key
    sortDir.value = key === 'consumed' || key === 'flc' ? 'desc' : 'asc'
  }
}

function startEdit(item: ItemView, field: 'absorbed_kw' | 'rated_kw', event: MouseEvent) {
  event.stopPropagation()
  editing.id = item.id
  editing.field = field
  editValue.value = String(field === 'absorbed_kw' ? item.absorbed_kw : item.rated_kw)
  nextTick(() => {
    const input = document.querySelector<HTMLInputElement>('[data-inline-edit]')
    input?.focus()
    input?.select()
  })
}

function cancelEdit() {
  editing.id = ''
  editing.field = null
}

async function commitEdit(item: ItemView) {
  if (!editing.field || editing.id !== item.id) return
  const field = editing.field
  const parsed = Number(editValue.value.replace(',', '.'))
  if (!Number.isFinite(parsed) || parsed < 0) {
    cancelEdit()
    return
  }
  const { derived: _derived, ...base } = item
  const next = { ...base, [field]: parsed }
  cancelEdit()
  if (next[field] === item[field]) return
  busyEdit.value = true
  await saveItem(next)
  busyEdit.value = false
}

async function doReset() {
  confirmReset.value = false
  await resetItems()
}

const activeFilterCount = computed(() => {
  const f = ui.filters
  return [f.area, f.voltage, f.duty, f.klass, f.status].filter(Boolean).length + (f.warningsOnly ? 1 : 0)
})

function warningTitle(item: ItemView): string {
  return item.derived.warnings.map((warning) => warning.message).join('\n')
}
</script>

<template>
  <div style="display: flex; flex-direction: column; gap: 14px">
    <div class="page-head">
      <div>
        <div class="eyebrow">Overall load list</div>
        <h1>Consumers</h1>
        <p>
          {{ fmt(activeInScenario.length, 0) }} consumers active in
          {{ SCENARIOS.find((entry) => entry.key === scenario)?.label }} · double-click a power cell to edit
        </p>
      </div>
      <div class="spacer" />
      <div class="spread">
        <button class="btn" :disabled="ui.busy" @click="confirmReset = true">Restore source data</button>
      </div>
    </div>

    <section class="card">
      <div class="card-head" style="flex-wrap: wrap; gap: 10px">
        <div class="search">
          <span class="icon">⌕</span>
          <input
            v-model="ui.filters.search"
            data-search
            class="input"
            placeholder="Search tag, description, system…  ( / )"
          />
        </div>

        <div class="toolbar">
          <select v-model="ui.filters.area" class="select sm">
            <option value="">All areas</option>
            <option v-for="area in state?.options.areas ?? []" :key="area" :value="area">
              Area {{ area }}
            </option>
          </select>

          <select v-model="ui.filters.voltage" class="select sm">
            <option value="">All voltages</option>
            <option v-for="voltage in state?.options.voltages ?? []" :key="voltage" :value="voltage">
              {{ Number(voltage.toFixed(2)) }} kV
            </option>
          </select>

          <select v-model="ui.filters.duty" class="select sm">
            <option value="">All duties</option>
            <option v-for="duty in DUTIES" :key="duty.key" :value="duty.key">{{ duty.label }}</option>
          </select>

          <select v-model="ui.filters.klass" class="select sm">
            <option value="">All classes</option>
            <option v-for="entry in CLASSES" :key="entry.key" :value="entry.key">
              {{ entry.label }}
            </option>
          </select>

          <select v-model="ui.filters.status" class="select sm">
            <option value="">All statuses</option>
            <option v-for="status in state?.options.statuses ?? []" :key="status" :value="status">
              {{ status }}
            </option>
          </select>

          <label class="check" :class="{ on: ui.filters.warningsOnly }">
            <input v-model="ui.filters.warningsOnly" type="checkbox" />
            Findings only
          </label>

          <button v-if="activeFilterCount || ui.filters.search" class="btn sm" @click="resetFilters">
            Clear
          </button>
        </div>

        <div class="spacer" />
        <span class="pill neutral">{{ fmt(sorted.length, 0) }} rows</span>
      </div>

      <div class="table-wrap" style="max-height: calc(100vh - 330px); min-height: 320px">
        <table class="grid-table">
          <thead>
            <tr>
              <th style="width: 54px" class="num" @click="toggleSort('line_no')">
                Line
                <span v-if="sortKey === 'line_no'">{{ sortDir === 'asc' ? '↑' : '↓' }}</span>
              </th>
              <th style="min-width: 140px" @click="toggleSort('tag')">
                Tag <span v-if="sortKey === 'tag'">{{ sortDir === 'asc' ? '↑' : '↓' }}</span>
              </th>
              <th style="min-width: 240px">Description</th>
              <th style="width: 62px">Area</th>
              <th style="width: 72px">Class</th>
              <th style="width: 92px">Duty</th>
              <th class="num" style="width: 74px">V (kV)</th>
              <th class="num" style="width: 96px">Absorbed</th>
              <th class="num" style="width: 84px">Rated</th>
              <th class="num" style="width: 66px">LF</th>
              <th class="num" style="width: 62px">η</th>
              <th class="num" style="width: 62px">cos φ</th>
              <th class="num" style="width: 82px" @click="toggleSort('flc')">
                FLC (A) <span v-if="sortKey === 'flc'">{{ sortDir === 'asc' ? '↑' : '↓' }}</span>
              </th>
              <th class="num" style="width: 92px" @click="toggleSort('consumed')">
                kW <span v-if="sortKey === 'consumed'">{{ sortDir === 'asc' ? '↑' : '↓' }}</span>
              </th>
              <th class="num" style="width: 84px">kVAr</th>
              <th style="width: 96px">Scenarios</th>
              <th style="width: 40px"></th>
            </tr>
          </thead>
          <tbody>
            <tr
              v-for="item in paged"
              :key="item.id"
              class="row-click"
              :class="{ selected: ui.drawerId === item.id }"
              @click="ui.drawerId = item.id"
            >
              <td class="num dim">{{ item.line_no ?? '–' }}</td>
              <td class="mono">{{ item.tag ?? '–' }}</td>
              <td :title="item.description ?? ''">
                <span>{{ item.description ?? '—' }}</span>
                <span v-if="item.remark" class="muted small"> · {{ item.remark }}</span>
              </td>
              <td class="dim">{{ item.area ?? '–' }}</td>
              <td>
                <span class="pill neutral">{{ item.equipment_class }}</span>
              </td>
              <td>
                <span class="pill" :class="item.duty ?? 'unassigned'">{{ dutyLabel(item.duty) }}</span>
              </td>
              <td class="num dim">{{ fmtSmart(item.voltage_kv) }}</td>
              <td class="num">
                <input
                  v-if="editing.id === item.id && editing.field === 'absorbed_kw'"
                  v-model="editValue"
                  data-inline-edit
                  class="input sm num"
                  type="number"
                  step="0.1"
                  @click.stop
                  @keydown.enter.prevent="commitEdit(item)"
                  @keydown.esc.prevent="cancelEdit"
                  @blur="commitEdit(item)"
                />
                <span
                  v-else
                  class="nowrap"
                  style="border-bottom: 1px dashed var(--line-strong); cursor: text"
                  title="Double-click to edit"
                  @dblclick="startEdit(item, 'absorbed_kw', $event)"
                >
                  {{ fmtSmart(item.absorbed_kw) }}
                </span>
              </td>
              <td class="num dim">
                <input
                  v-if="editing.id === item.id && editing.field === 'rated_kw'"
                  v-model="editValue"
                  data-inline-edit
                  class="input sm num"
                  type="number"
                  step="0.1"
                  @click.stop
                  @keydown.enter.prevent="commitEdit(item)"
                  @keydown.esc.prevent="cancelEdit"
                  @blur="commitEdit(item)"
                />
                <span
                  v-else
                  class="nowrap"
                  style="border-bottom: 1px dashed var(--line-strong); cursor: text"
                  title="Double-click to edit"
                  @dblclick="startEdit(item, 'rated_kw', $event)"
                >
                  {{ fmtSmart(item.rated_kw) }}
                </span>
              </td>
              <td class="num dim">{{ fmtPct(item.derived.load_factor, 0) }}</td>
              <td class="num dim">{{ fmt(item.derived.efficiency, 3) }}</td>
              <td class="num dim">{{ fmt(item.derived.power_factor, 3) }}</td>
              <td class="num dim">{{ fmt(item.derived.flc_a, 1) }}</td>
              <td class="num strong">{{ fmt(item.derived.consumed_kw, 1) }}</td>
              <td class="num dim">{{ fmt(item.derived.consumed_kvar, 1) }}</td>
              <td>
                <span class="scenario-dots" :title="item.scenarios.map((s) => SCENARIOS.find((e) => e.key === s)?.short).join(', ')">
                  <i
                    v-for="entry in SCENARIOS"
                    :key="entry.key"
                    :class="{ on: item.scenarios.includes(entry.key) }"
                  />
                </span>
              </td>
              <td>
                <span v-if="item.derived.warnings.length" class="warn-icon" :title="warningTitle(item)">!</span>
              </td>
            </tr>
            <tr v-if="!paged.length">
              <td colspan="17">
                <div class="empty">
                  No consumer matches the current filters.
                  <button class="btn sm" style="margin-left: 8px" @click="resetFilters">Clear filters</button>
                </div>
              </td>
            </tr>
          </tbody>
          <tfoot>
            <tr>
              <td colspan="7" class="small muted">
                {{ activeInScenario.length }} active in
                {{ SCENARIOS.find((entry) => entry.key === scenario)?.short }}
              </td>
              <td colspan="6" class="num small muted">
                Σ C {{ fmt(footerTotals.continuous, 0) }} · I {{ fmt(footerTotals.intermittent, 0) }} ·
                SB {{ fmt(footerTotals.standby, 0) }} kW
              </td>
              <td class="num strong">{{ fmt(footerTotals.consumed, 0) }}</td>
              <td colspan="3" class="small muted">
                connected · demand {{ fmt(summary?.demand.kw ?? 0, 0) }} kW
              </td>
            </tr>
          </tfoot>
        </table>
      </div>

      <div class="card-head" style="border-top: 1px solid var(--line-soft); border-bottom: none">
        <span class="small muted">{{ rangeLabel }}</span>
        <div class="spacer" />
        <select v-model.number="ui.pageSize" class="select sm" style="width: 92px">
          <option :value="50">50 / page</option>
          <option :value="100">100 / page</option>
          <option :value="250">250 / page</option>
          <option :value="500">500 / page</option>
        </select>
        <button class="btn sm" :disabled="ui.page <= 1" @click="ui.page -= 1">←</button>
        <span class="small mono">{{ ui.page }} / {{ pageCount }}</span>
        <button class="btn sm" :disabled="ui.page >= pageCount" @click="ui.page += 1">→</button>
      </div>
    </section>

    <div v-if="busyEdit" class="small muted">Saving…</div>

    <ConfirmDialog
      v-if="confirmReset"
      title="Restore source data?"
      message="Every manual change will be discarded and the load list will be rebuilt from the original workbook data embedded in the service. This cannot be undone."
      confirm-label="Restore"
      danger
      :busy="ui.busy"
      @confirm="doReset"
      @cancel="confirmReset = false"
    />
  </div>
</template>
