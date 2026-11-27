<script setup lang="ts">
import { computed } from 'vue'
import BarList from '../components/BarList.vue'
import StackedBar from '../components/StackedBar.vue'
import StatCard from '../components/StatCard.vue'
import { fmt, fmtPct } from '../lib/format'
import { currentSummary, data, setScenario, ui } from '../store'
import { DUTY_COLORS, SCENARIOS, scenarioLabel } from '../types'

const summary = currentSummary
const settings = computed(() => data.state?.settings)
const summaries = computed(() => data.state?.summaries ?? [])

const intFactor = computed(() => (settings.value?.intermittent_factor_pct ?? 30) / 100)
const stdFactor = computed(() => (settings.value?.standby_factor_pct ?? 10) / 100)
const contingency = computed(() => settings.value?.contingency_pct ?? 20)

const governingStandby = computed(() => {
  if (!summary.value) return 0
  return Math.max(summary.value.standby.kw * stdFactor.value, summary.value.largest.standby.kw)
})

const peakSegments = computed(() => {
  if (!summary.value) return []
  return [
    {
      key: 'continuous',
      label: 'Continuous',
      value: summary.value.continuous.kw,
      color: DUTY_COLORS.continuous,
      text: `${fmt(summary.value.continuous.kw)} kW`,
    },
    {
      key: 'intermittent',
      label: 'Intermittent (diversified)',
      value: summary.value.intermittent.kw * intFactor.value,
      color: DUTY_COLORS.intermittent,
      text: `${fmt(summary.value.intermittent.kw * intFactor.value)} kW`,
    },
    {
      key: 'standby',
      label: 'Governing stand-by',
      value: governingStandby.value,
      color: DUTY_COLORS.standby,
      text: `${fmt(governingStandby.value)} kW`,
    },
  ]
})

const voltageRows = computed(() =>
  (summary.value?.by_voltage ?? []).map((row) => ({
    key: row.key,
    value: row.connected_kw,
    note: `${row.items} items`,
  })),
)

const areaRows = computed(() =>
  (summary.value?.by_area ?? []).map((row) => ({
    key:
      row.key === 'T'
        ? 'T — Topside'
        : row.key === 'H'
          ? 'H — Hull'
          : row.key === 'L'
            ? 'L — LQ'
            : row.key,
    value: row.connected_kw,
    note: `${row.items} items`,
  })),
)

const maxDemand = computed(() => Math.max(1, ...summaries.value.map((entry) => entry.demand.kw)))

const extras = computed(() => {
  if (!summary.value) return null
  const entry = summary.value
  const gross = entry.continuous.kw + entry.intermittent.kw + entry.standby.kw
  return {
    continuousShare: entry.demand.kw > 0 ? entry.continuous.kw / entry.demand.kw : 0,
    diversityGain: gross - entry.peak.kw,
    demandRatio: entry.continuous.kw > 0 ? entry.demand.kw / entry.continuous.kw : 0,
    spare: entry.spare.kw,
  }
})

const warningSummary = computed(() => {
  const counts = new Map<string, number>()
  for (const item of data.state?.items ?? []) {
    for (const warning of item.derived.warnings) {
      counts.set(warning.code, (counts.get(warning.code) ?? 0) + 1)
    }
  }
  const labels: Record<string, string> = {
    no_duty: 'No service duty assigned',
    no_scenario: 'No operating scenario selected',
    motor_not_in_table: 'Motor rating missing from reference table',
    load_factor_high: 'Absorbed power above nameplate rating',
    zero_power: 'No power entered',
  }
  return [...counts.entries()]
    .map(([code, count]) => ({ code, count, label: labels[code] ?? code }))
    .sort((a, b) => b.count - a.count)
})

function reviewWarnings() {
  ui.filters.warningsOnly = true
  ui.view = 'loadlist'
}
</script>

<template>
  <div v-if="summary" style="display: flex; flex-direction: column; gap: 18px">
    <div class="page-head">
      <div>
        <div class="eyebrow">Appendix 1 · Load summary</div>
        <h1>{{ scenarioLabel(summary.scenario) }} operation</h1>
        <p>
          {{ summary.active_items }} active consumers · contingency {{ fmt(contingency, 0) }} % ·
          diversity {{ fmt(intFactor * 100, 0) }} % intermittent / {{ fmt(stdFactor * 100, 0) }} % stand-by
        </p>
      </div>
      <div class="spacer" />
      <div class="spread">
        <a
          class="btn"
          :href="`/api/export/pdf?scenario=${summary.scenario}`"
          download
          title="Download this scenario as a PDF report"
        >
          ↓ PDF report
        </a>
        <button class="btn" @click="ui.view = 'method'">Method &amp; basis</button>
        <button class="btn" @click="ui.settingsOpen = true">⚙ Factors</button>
      </div>
    </div>

    <div class="grid cols-4">
      <StatCard
        label="Total load demand"
        :value="summary.demand.kva"
        unit="kVA"
        :secondary="`${fmt(summary.demand.kw, 0)} kW`"
        :note="`Peak + ${fmt(contingency, 0)} % contingency`"
      />
      <StatCard
        label="Peak load"
        :value="summary.peak.kw"
        unit="kW"
        accent="amber"
        :secondary="`${fmt(summary.peak.kvar, 0)} kVAr`"
        :note="`cos φ ${fmt(summary.peak_power_factor, 3)}`"
      />
      <StatCard
        label="Maximum normal running"
        :value="summary.max_normal_running.kw"
        unit="kW"
        accent="violet"
        :secondary="`${fmt(summary.max_normal_running.kva, 0)} kVA`"
        note="Continuous + diversified intermittent"
      />
      <StatCard
        label="Continuous load"
        :value="summary.continuous.kw"
        unit="kW"
        accent="green"
        :secondary="`${fmt(summary.continuous.kva, 0)} kVA`"
        :note="`${fmt(summary.continuous.kvar, 0)} kVAr`"
      />
    </div>

    <div class="grid dash">
      <section class="card">
        <div class="card-head">
          <h3>Load build-up</h3>
          <span class="sub">{{ summary.label }}</span>
          <div class="spacer" />
          <span class="pill neutral">kW / kVAr / kVA</span>
        </div>
        <div class="table-wrap">
          <table class="grid-table">
            <thead>
              <tr>
                <th>Load category</th>
                <th class="num">kW</th>
                <th class="num">kVAr</th>
                <th class="num">kVA</th>
                <th>Basis</th>
              </tr>
            </thead>
            <tbody>
              <tr>
                <td>
                  <span class="pill continuous"><i class="dot" />Continuous</span>
                </td>
                <td class="num strong">{{ fmt(summary.continuous.kw) }}</td>
                <td class="num">{{ fmt(summary.continuous.kvar) }}</td>
                <td class="num">{{ fmt(summary.continuous.kva) }}</td>
                <td class="dim small">All loads running continuously</td>
              </tr>
              <tr>
                <td>
                  <span class="pill intermittent"><i class="dot" />Intermittent</span>
                </td>
                <td class="num strong">{{ fmt(summary.intermittent.kw) }}</td>
                <td class="num">{{ fmt(summary.intermittent.kvar) }}</td>
                <td class="num">{{ fmt(summary.intermittent.kva) }}</td>
                <td class="dim small">
                  {{ fmt(intFactor * 100, 0) }} % applied or largest
                  ({{ fmt(summary.largest.intermittent.kw) }} kW)
                </td>
              </tr>
              <tr>
                <td>
                  <span class="pill standby"><i class="dot" />Stand-by</span>
                </td>
                <td class="num strong">{{ fmt(summary.standby.kw) }}</td>
                <td class="num">{{ fmt(summary.standby.kvar) }}</td>
                <td class="num">{{ fmt(summary.standby.kva) }}</td>
                <td class="dim small">
                  Largest governs ({{ fmt(summary.largest.standby.kw) }} kW)
                </td>
              </tr>
              <tr>
                <td>
                  <span class="pill spare"><i class="dot" />Spare</span>
                </td>
                <td class="num strong">{{ fmt(summary.spare.kw) }}</td>
                <td class="num">{{ fmt(summary.spare.kvar) }}</td>
                <td class="num">{{ fmt(summary.spare.kva) }}</td>
                <td class="dim small">Not included in the peak load</td>
              </tr>
              <tr>
                <td class="strong">Maximum normal running load</td>
                <td class="num strong">{{ fmt(summary.max_normal_running.kw) }}</td>
                <td class="num">{{ fmt(summary.max_normal_running.kvar) }}</td>
                <td class="num">{{ fmt(summary.max_normal_running.kva) }}</td>
                <td class="dim small">Continuous + diversified intermittent</td>
              </tr>
              <tr>
                <td class="strong">Peak load</td>
                <td class="num strong">{{ fmt(summary.peak.kw) }}</td>
                <td class="num">{{ fmt(summary.peak.kvar) }}</td>
                <td class="num">{{ fmt(summary.peak.kva) }}</td>
                <td class="dim small">Running load + governing stand-by</td>
              </tr>
              <tr>
                <td class="strong">Total load demand</td>
                <td class="num strong" style="color: var(--brand-dark)">{{ fmt(summary.demand.kw) }}</td>
                <td class="num">{{ fmt(summary.demand.kvar) }}</td>
                <td class="num strong" style="color: var(--brand-dark)">{{ fmt(summary.demand.kva) }}</td>
                <td class="dim small">Peak × (1 + {{ fmt(contingency, 0) }} %)</td>
              </tr>
            </tbody>
          </table>
        </div>
        <div
          v-if="extras"
          class="card-body"
          style="
            display: grid;
            grid-template-columns: repeat(4, minmax(0, 1fr));
            gap: 12px;
            border-top: 1px solid var(--line-soft);
          "
        >
          <div>
            <div class="eyebrow">Continuous share</div>
            <div class="mono" style="font-size: 15px">{{ fmtPct(extras.continuousShare, 0) }}</div>
            <div class="small muted">of total demand</div>
          </div>
          <div>
            <div class="eyebrow">Diversity gain</div>
            <div class="mono" style="font-size: 15px">{{ fmt(extras.diversityGain, 0) }} kW</div>
            <div class="small muted">vs. sum of all duties</div>
          </div>
          <div>
            <div class="eyebrow">Demand / continuous</div>
            <div class="mono" style="font-size: 15px">{{ fmt(extras.demandRatio, 2) }} ×</div>
            <div class="small muted">generation margin</div>
          </div>
          <div>
            <div class="eyebrow">Spare installed</div>
            <div class="mono" style="font-size: 15px">{{ fmt(extras.spare, 0) }} kW</div>
            <div class="small muted">not in the peak</div>
          </div>
        </div>
      </section>

      <div style="display: flex; flex-direction: column; gap: 14px">
        <section class="card">
          <div class="card-head">
            <h3>Peak composition</h3>
            <div class="spacer" />
            <span class="sub">cos φ {{ fmt(summary.peak_power_factor, 3) }}</span>
          </div>
          <div class="card-body">
            <StackedBar :segments="peakSegments" :height="30">
              <template #legend>
                <span v-for="segment in peakSegments" :key="segment.key" class="item">
                  <i class="swatch" :style="{ background: segment.color }" />
                  {{ segment.label }}
                  <span class="value">{{ fmt(segment.value) }}</span>
                </span>
              </template>
            </StackedBar>
            <div class="divider" style="margin: 14px 0 10px" />
            <dl class="kv">
              <dt>Largest continuous consumer</dt>
              <dd>{{ fmt(summary.largest.continuous.kw) }} kW</dd>
              <dt>Largest stand-by consumer</dt>
              <dd>{{ fmt(summary.largest.standby.kw) }} kW</dd>
              <dt>Spare installed capacity</dt>
              <dd class="muted">{{ fmt(summary.spare.kw) }} kW</dd>
            </dl>
          </div>
        </section>

        <section class="card">
          <div class="card-head">
            <h3>Generator sizing</h3>
            <div class="spacer" />
            <span class="sub">indicative</span>
          </div>
          <div class="table-wrap">
            <table class="grid-table">
              <thead>
                <tr>
                  <th>Configuration</th>
                  <th class="num">Units</th>
                  <th class="num">Unit kW</th>
                  <th class="num">Unit kVA</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="option in summary.generator_sizing" :key="option.label">
                  <td>{{ option.label }}</td>
                  <td class="num dim">{{ option.running }}/{{ option.units }}</td>
                  <td class="num strong">{{ fmt(option.unit_kw, 0) }}</td>
                  <td class="num">{{ fmt(option.unit_kva, 0) }}</td>
                </tr>
              </tbody>
            </table>
          </div>
          <div class="card-body small muted" style="padding-top: 10px">
            Before site ambient derating, engine margins and motor start contribution.
          </div>
        </section>
      </div>
    </div>

    <div class="grid cols-2">
      <section class="card">
        <div class="card-head">
          <h3>Connected load by voltage</h3>
          <div class="spacer" />
          <span class="sub">{{ summary.label }}</span>
        </div>
        <div class="card-body">
          <BarList :rows="voltageRows" unit="kW" />
          <div class="small muted" style="margin-top: 10px">
            Connected load of every active consumer, all duties combined.
          </div>
        </div>
      </section>

      <section class="card">
        <div class="card-head">
          <h3>Connected load by area</h3>
          <div class="spacer" />
          <span class="sub">{{ summary.label }}</span>
        </div>
        <div class="card-body">
          <BarList :rows="areaRows" unit="kW" />
          <div class="small muted" style="margin-top: 10px">
            Area code as recorded in the source load list.
          </div>
        </div>
      </section>
    </div>

    <div class="grid cols-2">
      <section class="card">
        <div class="card-head">
          <h3>Scenario comparison</h3>
          <div class="spacer" />
          <span class="sub">total demand, kW</span>
        </div>
        <div class="card-body">
          <div class="barlist">
            <button
              v-for="entry in summaries"
              :key="entry.scenario"
              class="row"
              style="
                border: none;
                background: transparent;
                padding: 0;
                cursor: pointer;
                text-align: left;
                grid-template-columns: 108px minmax(0, 1fr) 90px;
              "
              @click="setScenario(entry.scenario)"
            >
              <span class="label" :style="{ color: entry.scenario === summary.scenario ? 'var(--copper)' : undefined }">
                {{ SCENARIOS.find((s) => s.key === entry.scenario)?.short }}
              </span>
              <div class="track">
                <div
                  class="fill"
                  :style="{
                    width: `${Math.max(1.5, (entry.demand.kw / maxDemand) * 100)}%`,
                    background:
                      entry.scenario === summary.scenario
                        ? 'linear-gradient(90deg, var(--brand), var(--brand-2))'
                        : 'linear-gradient(90deg, #e2e8f0, #cbd5e1)',
                  }"
                />
              </div>
              <span class="value">{{ fmt(entry.demand.kw, 0) }}</span>
            </button>
          </div>
        </div>
      </section>

      <section class="card">
        <div class="card-head">
          <h3>Data quality</h3>
          <div class="spacer" />
          <span
            class="pill"
            :class="data.state?.stats.warnings ? 'unassigned' : 'neutral'"
          >
            {{ data.state?.stats.warnings ?? 0 }} findings
          </span>
        </div>
        <div class="card-body" style="display: flex; flex-direction: column; gap: 10px">
          <div v-if="!warningSummary.length" class="small muted">
            Every consumer has a duty, a scenario and a valid efficiency basis.
          </div>
          <div v-for="entry in warningSummary" :key="entry.code" class="spread">
            <span class="small">{{ entry.label }}</span>
            <span class="pill neutral">{{ entry.count }}</span>
          </div>
          <button v-if="warningSummary.length" class="btn sm" style="align-self: flex-start" @click="reviewWarnings">
            Review flagged items
          </button>
        </div>
      </section>
    </div>
  </div>
</template>
