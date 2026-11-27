<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { computeCathodicDerived, methodLabel, scopeLabel } from '../lib/cathodic'
import { fmt } from '../lib/format'
import {
  cathodicDrawerItem,
  deleteCathodicItem,
  duplicateCathodicItem,
  saveCathodicItem,
  ui,
} from '../store'
import {
  CATHODIC_METHODS,
  CATHODIC_ORIENTATIONS,
  CATHODIC_STRUCTURES,
  CATHODIC_SURFACES,
  type CathodicItem,
  type CathodicRow,
  type RectifierInputs,
} from '../types'
import ConfirmDialog from './ConfirmDialog.vue'

const draft = ref<CathodicItem | null>(null)
const confirmDelete = ref(false)
const saving = ref(false)

function defaultRectifier(): RectifierInputs {
  return {
    anode_output_a: 5,
    groundbed_resistance_ohm: 1,
    cable_resistance_ohm: 0.2,
    back_emf_v: 2,
    margin_pct: 20,
  }
}

function strip(row: CathodicRow): CathodicItem {
  const copy = JSON.parse(JSON.stringify(row)) as Record<string, unknown>
  delete copy.derived
  return copy as unknown as CathodicItem
}

function normalize(item: CathodicItem): CathodicItem {
  const copy = JSON.parse(JSON.stringify(item)) as CathodicItem
  if (copy.capacity_temp_c === null || !Number.isFinite(copy.capacity_temp_c)) {
    copy.capacity_temp_c = copy.operating_temp_c
  }
  if (copy.anode_output_a !== null && !Number.isFinite(copy.anode_output_a)) copy.anode_output_a = null
  copy.anodes_selected = Math.max(1, Math.round(copy.anodes_selected || 1))
  if (copy.method === 'iccp' && !copy.rectifier) copy.rectifier = defaultRectifier()
  return copy
}

watch(
  () => cathodicDrawerItem.value,
  (row) => {
    if (row) draft.value = normalize(strip(row))
  },
  { immediate: true },
)

const derived = computed(() => (draft.value ? computeCathodicDerived(draft.value) : null))

const dirty = computed(() => {
  if (!draft.value || !cathodicDrawerItem.value) return false
  return (
    JSON.stringify(normalize(draft.value)) !== JSON.stringify(normalize(strip(cathodicDrawerItem.value)))
  )
})

const isPipe = computed(() => draft.value?.structure === 'piping' || draft.value?.structure === 'pipeline')

function onMethodChange() {
  if (!draft.value) return
  if (draft.value.method === 'iccp' && !draft.value.rectifier) {
    draft.value.rectifier = defaultRectifier()
  }
}

async function save() {
  if (!draft.value || saving.value) return
  saving.value = true
  const result = await saveCathodicItem(normalize(draft.value))
  saving.value = false
  if (result && cathodicDrawerItem.value) draft.value = normalize(strip(cathodicDrawerItem.value))
}

async function duplicate() {
  if (!cathodicDrawerItem.value) return
  const copy = await duplicateCathodicItem(cathodicDrawerItem.value.id)
  if (copy) ui.cathodicDrawerId = copy.id
}

async function remove() {
  if (!cathodicDrawerItem.value) return
  const id = cathodicDrawerItem.value.id
  confirmDelete.value = false
  await deleteCathodicItem(id)
  ui.cathodicDrawerId = null
}
</script>

<template>
  <div v-if="draft && derived">
    <div class="drawer-backdrop" @click="ui.cathodicDrawerId = null" />
    <aside class="drawer">
      <div class="drawer-head">
        <div style="min-width: 0">
          <h2>{{ draft.tag || 'New protected item' }}</h2>
          <div class="tag">{{ scopeLabel(draft) }} · {{ methodLabel(draft.method) }}</div>
        </div>
        <div class="spacer" style="margin-left: auto" />
        <button class="btn icon" title="Close (Esc)" @click="ui.cathodicDrawerId = null">✕</button>
      </div>

      <div class="drawer-body">
        <div class="computed">
          <div class="big">
            <b>{{ derived.anodes_required }}</b>
            <span>anodes required</span>
            <span style="margin-left: auto" class="mono">{{ derived.anodes_selected }} selected</span>
          </div>
          <div class="small muted">
            {{ fmt(derived.exposed_area_m2, 1) }} m² exposed · {{ fmt(derived.protection_current_a, 2) }} A ·
            Cdu {{ fmt(derived.current_density_a_m2, 5) }} A/m²
          </div>

          <div class="divider" style="margin: 12px 0" />

          <dl class="kv">
            <dt>Capacity at temperature</dt>
            <dd>{{ fmt(derived.capacity_at_temp_ah_kg, 0) }} Ah/kg</dd>
            <dt>Wetted / exposed area</dt>
            <dd>{{ fmt(derived.wetted_area_m2, 1) }} → {{ fmt(derived.exposed_area_m2, 1) }} m²</dd>
            <template v-if="draft.method === 'sacp'">
              <dt>Anode mass</dt>
              <dd>{{ derived.anode_mass_kg === null ? '–' : `${fmt(derived.anode_mass_kg, 1)} kg` }}</dd>
              <dt>By mass / by current</dt>
              <dd>
                {{ derived.anodes_by_mass ?? '–' }} / {{ derived.anodes_by_current ?? '–' }}
              </dd>
              <dt>Life from selected anodes</dt>
              <dd>{{ derived.life_years === null ? '–' : `${fmt(derived.life_years, 1)} years` }}</dd>
            </template>
            <template v-else>
              <dt>Rectifier current</dt>
              <dd>{{ fmt(derived.rectifier_current_a ?? 0, 1) }} A</dd>
              <dt>Rectifier voltage / rating</dt>
              <dd>
                {{ fmt(derived.rectifier_voltage_v ?? 0, 1) }} V ·
                {{ fmt(derived.rectifier_kva ?? 0, 2) }} kVA
              </dd>
            </template>
          </dl>

          <div class="formula" style="margin-top: 12px">
            Cdu = ({{ fmt(draft.current_density_a_m2, 4) }} +
            ({{ fmt(draft.design_temp_c, 1) }} − 25) × {{ fmt(draft.temp_coefficient, 4) }}) ×
            {{ draft.apply_coating_breakdown ? fmt(draft.coating_breakdown, 3) : '1' }} =
            {{ fmt(derived.current_density_a_m2, 5) }} A/m²<br />
            Ip = {{ fmt(derived.exposed_area_m2, 2) }} × {{ fmt(derived.current_density_a_m2, 5) }} =
            {{ fmt(derived.protection_current_a, 3) }} A
            <template v-if="draft.method === 'sacp'">
              <br />
              M = Ip × {{ fmt(draft.design_life_years, 1) }} × 8760 /
              {{ fmt(derived.capacity_at_temp_ah_kg, 1) }} / {{ fmt(draft.utilization, 2) }} ×
              {{ fmt(draft.safety_factor, 2) }} =
              {{ derived.anode_mass_kg === null ? '–' : `${fmt(derived.anode_mass_kg, 2)} kg` }}
            </template>
          </div>

          <div style="display: flex; gap: 6px; flex-wrap: wrap; margin-top: 12px">
            <span class="pill" :class="derived.checks.capacity ? 'continuous' : 'unassigned'">Capacity</span>
            <span class="pill" :class="derived.checks.current_density ? 'continuous' : 'unassigned'">
              Current density
            </span>
            <span class="pill" :class="derived.checks.anode_count ? 'continuous' : 'unassigned'">
              Anode count
            </span>
            <span class="pill" :class="derived.checks.design_life ? 'continuous' : 'unassigned'">
              Design life
            </span>
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
              <label>Tag</label>
              <input v-model="draft.tag" class="input" placeholder="T-2430" />
            </div>
            <div class="field">
              <label>Description</label>
              <input v-model="draft.description" class="input" placeholder="Internal tank bottom" />
            </div>
          </div>
        </div>

        <div>
          <div class="section-title">Scope</div>
          <div class="form-grid">
            <div class="field">
              <label>Structure</label>
              <select v-model="draft.structure" class="select">
                <option v-for="entry in CATHODIC_STRUCTURES" :key="entry.key" :value="entry.key">
                  {{ entry.label }}
                </option>
              </select>
            </div>
            <div class="field">
              <label>Orientation</label>
              <select v-model="draft.orientation" class="select" :disabled="isPipe">
                <option v-for="entry in CATHODIC_ORIENTATIONS" :key="entry.key" :value="entry.key">
                  {{ entry.label }}
                </option>
              </select>
            </div>
            <div class="field">
              <label>Surface</label>
              <select v-model="draft.surface" class="select">
                <option v-for="entry in CATHODIC_SURFACES" :key="entry.key" :value="entry.key">
                  {{ entry.label }}
                </option>
              </select>
            </div>
            <div class="field">
              <label>Protection method</label>
              <select v-model="draft.method" class="select" @change="onMethodChange">
                <option v-for="entry in CATHODIC_METHODS" :key="entry.key" :value="entry.key">
                  {{ entry.label }}
                </option>
              </select>
            </div>
          </div>
        </div>

        <div>
          <div class="section-title">Design basis</div>
          <div class="form-grid">
            <div class="field">
              <label>Design life (years)</label>
              <input v-model.number="draft.design_life_years" class="input num" type="number" step="1" min="0" />
            </div>
            <div class="field">
              <label>Coating breakdown CBf</label>
              <input v-model.number="draft.coating_breakdown" class="input num" type="number" step="0.01" min="0" />
            </div>
            <div class="field">
              <label>Mean current density (A/m²)</label>
              <input v-model.number="draft.current_density_a_m2" class="input num" type="number" step="0.001" min="0" />
            </div>
            <div class="field">
              <label>Temperature coefficient (/°C)</label>
              <input v-model.number="draft.temp_coefficient" class="input num" type="number" step="0.001" min="0" />
            </div>
            <div class="field">
              <label>Design temperature (°C)</label>
              <input v-model.number="draft.design_temp_c" class="input num" type="number" step="0.1" />
              <div class="small muted">Used for the current density correction.</div>
            </div>
            <div class="field">
              <label>Operating temperature (°C)</label>
              <input v-model.number="draft.operating_temp_c" class="input num" type="number" step="0.1" />
            </div>
            <div class="field">
              <label>Capacity temperature (°C)</label>
              <input v-model.number="draft.capacity_temp_c" class="input num" type="number" step="0.1" />
              <div class="small muted">Defaults to the operating temperature.</div>
            </div>
            <div class="field">
              <label>Anode capacity (Ah/kg)</label>
              <input v-model.number="draft.capacity_ah_kg" class="input num" type="number" step="1" min="0" />
            </div>
            <div class="field">
              <label>Utilization factor U</label>
              <input v-model.number="draft.utilization" class="input num" type="number" step="0.01" min="0" max="1" />
            </div>
            <div class="field">
              <label>Safety factor S</label>
              <input v-model.number="draft.safety_factor" class="input num" type="number" step="0.01" min="0.1" />
            </div>
            <div class="field">
              <label>Apply coating breakdown</label>
              <label class="small" style="display: flex; gap: 8px; align-items: center; padding-top: 6px">
                <input v-model="draft.apply_coating_breakdown" type="checkbox" />
                Multiply the temperature-corrected density by CBf
              </label>
            </div>
          </div>
        </div>

        <div>
          <div class="section-title">Geometry</div>
          <div class="form-grid">
            <div v-if="!isPipe && draft.orientation !== 'rectangular'" class="field">
              <label>Diameter (m)</label>
              <input v-model.number="draft.diameter_m" class="input num" type="number" step="0.01" min="0" />
            </div>
            <div v-if="isPipe" class="field">
              <label>Diameter (m)</label>
              <input v-model.number="draft.diameter_m" class="input num" type="number" step="0.01" min="0" />
            </div>
            <div class="field">
              <label>{{ isPipe ? 'Length (m)' : 'Length / height (m)' }}</label>
              <input v-model.number="draft.length_m" class="input num" type="number" step="0.01" min="0" />
            </div>
            <div v-if="draft.orientation === 'rectangular' && !isPipe" class="field">
              <label>Width (m)</label>
              <input v-model.number="draft.width_m" class="input num" type="number" step="0.01" min="0" />
            </div>
            <div v-if="!isPipe" class="field">
              <label>Liquid height (m)</label>
              <input v-model.number="draft.liquid_height_m" class="input num" type="number" step="0.01" min="0" />
              <div class="small muted">Internal wetted height; external surfaces use the full length.</div>
            </div>
            <div class="field">
              <label>Discrepancy allowance (%)</label>
              <input v-model.number="draft.discrepancy_pct" class="input num" type="number" step="0.1" min="0" />
            </div>
          </div>
        </div>

        <div>
          <div class="section-title">{{ draft.method === 'sacp' ? 'Anode' : 'Groundbed' }}</div>
          <div class="form-grid">
            <div class="field">
              <label>Anode type</label>
              <input v-model="draft.anode_type" class="input" placeholder="ICS 40 AWB" />
            </div>
            <div class="field">
              <label>Anode unit weight (kg)</label>
              <input v-model.number="draft.anode_weight_kg" class="input num" type="number" step="0.1" min="0" />
            </div>
            <div v-if="draft.method === 'sacp'" class="field">
              <label>Anode output (A)</label>
              <input v-model.number="draft.anode_output_a" class="input num" type="number" step="0.01" min="0" />
              <div class="small muted">Optional current check; blank skips it.</div>
            </div>
            <div class="field">
              <label>Anodes selected</label>
              <input v-model.number="draft.anodes_selected" class="input num" type="number" step="1" min="1" />
            </div>
          </div>
        </div>

        <div v-if="draft.method === 'iccp' && draft.rectifier">
          <div class="section-title">Rectifier &amp; groundbed</div>
          <div class="form-grid">
            <div class="field">
              <label>Groundbed anode output (A)</label>
              <input v-model.number="draft.rectifier.anode_output_a" class="input num" type="number" step="0.1" min="0" />
            </div>
            <div class="field">
              <label>Groundbed resistance (Ω)</label>
              <input v-model.number="draft.rectifier.groundbed_resistance_ohm" class="input num" type="number" step="0.01" min="0" />
            </div>
            <div class="field">
              <label>Cable resistance (Ω)</label>
              <input v-model.number="draft.rectifier.cable_resistance_ohm" class="input num" type="number" step="0.01" min="0" />
            </div>
            <div class="field">
              <label>Back EMF (V)</label>
              <input v-model.number="draft.rectifier.back_emf_v" class="input num" type="number" step="0.1" min="0" />
            </div>
            <div class="field">
              <label>Rectifier margin (%)</label>
              <input v-model.number="draft.rectifier.margin_pct" class="input num" type="number" step="1" min="0" />
            </div>
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
      title="Delete this protected item?"
      :message="`${draft.tag || 'This item'} will be removed from the cathodic schedule.`"
      confirm-label="Delete"
      danger
      @confirm="remove"
      @cancel="confirmDelete = false"
    />
  </div>
</template>
