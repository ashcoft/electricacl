<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { computeDerived } from '../lib/calc'
import { fmt, fmtPct, fmtSmart } from '../lib/format'
import { data, deleteItem, duplicateItem, drawerItem, saveItem, ui } from '../store'
import { CLASSES, DUTIES, SCENARIOS, type ClassKey, type LoadItem, type ScenarioKey } from '../types'
import ConfirmDialog from './ConfirmDialog.vue'

const draft = ref<LoadItem | null>(null)
const confirmDelete = ref(false)
const saving = ref(false)

const motorTable = computed(() => data.state?.motor_table ?? [])

function strip(item: LoadItem & { derived?: unknown }): LoadItem {
  const copy = JSON.parse(JSON.stringify(item)) as Record<string, unknown>
  delete copy.derived
  return copy as unknown as LoadItem
}

watch(
  () => drawerItem.value,
  (item) => {
    if (item) draft.value = strip(item)
  },
  { immediate: true },
)

const derived = computed(() => (draft.value ? computeDerived(draft.value, motorTable.value) : null))

const dirty = computed(() => {
  if (!draft.value || !drawerItem.value) return false
  return JSON.stringify(draft.value) !== JSON.stringify(strip(drawerItem.value))
})

function toggleScenario(key: ScenarioKey) {
  if (!draft.value) return
  const set = new Set(draft.value.scenarios)
  if (set.has(key)) set.delete(key)
  else set.add(key)
  draft.value.scenarios = SCENARIOS.filter((entry) => set.has(entry.key)).map((entry) => entry.key)
}

function onBreaker(event: Event) {
  if (!draft.value) return
  const raw = (event.target as HTMLInputElement).value
  draft.value.breaker_a = raw === '' ? null : Number(raw)
}

async function save() {
  if (!draft.value || saving.value) return
  saving.value = true
  const result = await saveItem(draft.value)
  saving.value = false
  if (result && drawerItem.value) draft.value = strip(drawerItem.value)
}

async function duplicate() {
  if (!drawerItem.value) return
  const copy = await duplicateItem(drawerItem.value.id)
  if (copy) ui.drawerId = copy.id
}

async function remove() {
  if (!drawerItem.value) return
  const id = drawerItem.value.id
  confirmDelete.value = false
  await deleteItem(id)
  ui.drawerId = null
}

const voltageOptions = computed(() => data.state?.options.voltages ?? [])
</script>

<template>
  <div v-if="draft && derived">
    <div class="drawer-backdrop" @click="ui.drawerId = null" />
    <aside class="drawer">
      <div class="drawer-head">
        <div style="min-width: 0">
          <h2>{{ draft.description || 'Untitled consumer' }}</h2>
          <div class="tag">
            {{ draft.tag || 'no tag' }} · {{ draft.equipment_class }} ·
            {{ draft.duty ?? 'unassigned' }}
          </div>
        </div>
        <div class="spacer" style="margin-left: auto" />
        <button class="btn icon" title="Close (Esc)" @click="ui.drawerId = null">✕</button>
      </div>

      <div class="drawer-body">
        <div class="computed">
          <div class="big">
            <b>{{ fmt(derived.consumed_kw, 2) }}</b>
            <span>kW consumed</span>
            <span style="margin-left: auto" class="mono">{{ fmt(derived.consumed_kva, 2) }} kVA</span>
          </div>
          <div class="small muted">
            {{ fmt(derived.consumed_kvar, 2) }} kVAr at cos φ {{ fmt(derived.power_factor, 3) }}
            <template v-if="derived.starting_kva">
              · start {{ fmt(derived.starting_kva, 0) }} kVA
            </template>
          </div>

          <div class="divider" style="margin: 12px 0" />

          <dl class="kv">
            <dt>Load factor</dt>
            <dd>{{ fmtPct(derived.load_factor, 1) }}</dd>
            <dt>Efficiency at load</dt>
            <dd>{{ fmt(derived.efficiency, 4) }}</dd>
            <dt>Power factor at load</dt>
            <dd>{{ fmt(derived.power_factor, 4) }}</dd>
            <dt>Full load current</dt>
            <dd>{{ fmt(derived.flc_a, 1) }} A</dd>
            <template v-if="derived.starting_current_a">
              <dt>Starting current</dt>
              <dd>{{ fmt(derived.starting_current_a, 0) }} A</dd>
            </template>
            <dt>Efficiency basis</dt>
            <dd>{{ derived.basis.replace('_', ' ') }}</dd>
          </dl>

          <div class="formula" style="margin-top: 12px">
            kW = absorbed / η = {{ fmtSmart(draft.absorbed_kw) }} / {{ fmt(derived.efficiency, 4) }} =
            {{ fmt(derived.consumed_kw, 2) }}<br />
            kVAr = kW · tan(acos φ) = {{ fmt(derived.consumed_kw, 2) }} · {{ fmt(derived.consumed_kw ? derived.consumed_kvar / derived.consumed_kw : 0, 4) }} =
            {{ fmt(derived.consumed_kvar, 2) }}<br />
            FLC = rated / (√3 · kV · η · cos φ) = {{ fmtSmart(draft.rated_kw) }} / (1.732 ·
            {{ fmtSmart(draft.voltage_kv) }} · {{ fmt(derived.efficiency, 4) }} ·
            {{ fmt(derived.power_factor, 4) }}) = {{ fmt(derived.flc_a, 1) }} A
          </div>
        </div>

        <div v-if="derived.warnings.length" class="warning-list">
          <div v-for="warning in derived.warnings" :key="warning.code" class="warning-item">
            <span>!</span>
            <span>{{ warning.message }}</span>
          </div>
        </div>

        <div>
          <div class="section-title">Identification</div>
          <div class="form-grid">
            <div class="field">
              <label>Tag number</label>
              <input v-model="draft.tag" class="input" placeholder="F-000-X-0001" />
            </div>
            <div class="field">
              <label>Line no.</label>
              <input v-model.number="draft.line_no" class="input num" type="number" min="0" />
            </div>
            <div class="field span-2">
              <label>Description</label>
              <input v-model="draft.description" class="input" placeholder="Consumer description" />
            </div>
            <div class="field">
              <label>Area</label>
              <select v-model="draft.area" class="select">
                <option :value="null">—</option>
                <option v-for="area in data.state?.options.areas ?? []" :key="area" :value="area">
                  {{ area }}
                </option>
              </select>
            </div>
            <div class="field">
              <label>System / sub-system</label>
              <input v-model="draft.system" class="input" placeholder="e.g. TEG regeneration" />
            </div>
            <div class="field">
              <label>Switchgear</label>
              <input v-model="draft.switchgear" class="input" placeholder="e.g. F-780-SB-1501" />
            </div>
            <div class="field">
              <label>Status</label>
              <input v-model="draft.status" class="input" placeholder="NEW / UPD / T-UPD" />
            </div>
          </div>
        </div>

        <div>
          <div class="section-title">Electrical data</div>
          <div class="form-grid">
            <div class="field span-2">
              <label>Equipment class</label>
              <div class="segmented">
                <button
                  v-for="entry in CLASSES"
                  :key="entry.key"
                  :class="{ active: draft.equipment_class === entry.key }"
                  @click="draft.equipment_class = entry.key as ClassKey"
                >
                  {{ entry.label }}
                </button>
              </div>
            </div>
            <div class="field">
              <label>Service duty</label>
              <select v-model="draft.duty" class="select">
                <option :value="null">Unassigned</option>
                <option v-for="entry in DUTIES" :key="entry.key" :value="entry.key">
                  {{ entry.label }}
                </option>
              </select>
            </div>
            <div class="field">
              <label>Starter / feeder type</label>
              <input v-model="draft.feeder_type" class="input" placeholder="DOL / VSD / SD / SS" />
            </div>
            <div class="field">
              <label>Rated voltage (kV)</label>
              <input v-model.number="draft.voltage_kv" class="input num" type="number" step="0.01" min="0" />
              <div class="small muted">
                Common: {{ voltageOptions.map((v) => `${Number(v.toFixed(2))} kV`).join(' · ') }}
              </div>
            </div>
            <div class="field">
              <label>Phase</label>
              <select v-model="draft.phase" class="select">
                <option :value="null">—</option>
                <option value="3PH">3PH</option>
                <option value="3N">3PH + N</option>
                <option value="1N">1PH + N</option>
                <option value="DC">DC</option>
              </select>
            </div>
            <div class="field">
              <label>Absorbed power (kW)</label>
              <input v-model.number="draft.absorbed_kw" class="input num" type="number" step="0.1" min="0" />
            </div>
            <div class="field">
              <label>Rated power (kW)</label>
              <input v-model.number="draft.rated_kw" class="input num" type="number" step="0.1" min="0" />
            </div>
            <div class="field">
              <label>Breaker rating (A)</label>
              <input
                :value="draft.breaker_a ?? ''"
                class="input num"
                type="number"
                step="1"
                min="0"
                @input="onBreaker"
              />
            </div>
            <div class="field">
              <label>Load type</label>
              <input v-model="draft.load_type" class="input" placeholder="Essential / Non-essential" />
            </div>
          </div>
        </div>

        <div>
          <div class="section-title">Operating scenarios</div>
          <div class="chip-row">
            <button
              v-for="entry in SCENARIOS"
              :key="entry.key"
              class="chip"
              :class="{ on: draft.scenarios.includes(entry.key) }"
              @click="toggleScenario(entry.key)"
            >
              {{ entry.label }}
            </button>
          </div>
          <div class="small muted" style="margin-top: 8px">
            The consumer is only counted in the scenarios selected here.
          </div>
        </div>

        <div>
          <div class="section-title">Remarks</div>
          <textarea v-model="draft.remark" class="input" placeholder="Basis of the data, references, notes…" />
        </div>
      </div>

      <div class="drawer-foot">
        <button class="btn danger" @click="confirmDelete = true">Delete</button>
        <button class="btn" @click="duplicate">Duplicate</button>
        <div class="spacer" style="margin-left: auto" />
        <button class="btn" :class="{ primary: dirty }" :disabled="!dirty || saving" @click="save">
          {{ saving ? 'Saving…' : dirty ? 'Save changes' : '✓ Saved' }}
        </button>
      </div>
    </aside>

    <ConfirmDialog
      v-if="confirmDelete"
      title="Delete this consumer?"
      :message="`${draft.tag || 'This load item'} will be removed from the load list and every scenario summary will be recalculated.`"
      confirm-label="Delete"
      danger
      @confirm="remove"
      @cancel="confirmDelete = false"
    />
  </div>
</template>
