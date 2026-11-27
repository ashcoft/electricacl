<script setup lang="ts">
import { computed } from 'vue'
import { fmt } from '../lib/format'
import { data } from '../store'

const summaries = computed(() => data.state?.summaries ?? [])
const settings = computed(() => data.state?.settings)

const workbook: Record<string, number> = {
  normal: 31140.746828032006,
  maintenance: 7833.7676590372375,
  essential: 11614.18717691474,
  emergency: 1929.787556240612,
}
</script>

<template>
  <div style="display: flex; flex-direction: column; gap: 14px">
    <div class="page-head">
      <div>
        <div class="eyebrow">Calculation basis</div>
        <h1>Method &amp; assumptions</h1>
        <p>How every number in this load list is produced, and how the engine is verified.</p>
      </div>
    </div>

    <div class="grid cols-2">
      <section class="card">
        <div class="card-head"><h3>Per-consumer calculation</h3></div>
        <div class="card-body" style="display: flex; flex-direction: column; gap: 12px">
          <div class="formula">
            load factor = absorbed power / nameplate rating<br />
            consumed kW = absorbed power / η<br />
            consumed kVAr = consumed kW · tan(acos(cos φ))<br />
            FLC = rated kW / (√3 · kV · η · cos φ)
          </div>
          <p class="small muted" style="line-height: 1.65">
            η and cos φ depend on the equipment class. Motors are interpolated from the Appendix-11
            reference curve; feeders use 0.95 / 0.90 and other electrical users 1.00 / 1.00.
          </p>
          <div class="table-wrap">
            <table class="grid-table">
              <thead>
                <tr>
                  <th>Load factor</th>
                  <th>Efficiency</th>
                  <th>Power factor</th>
                </tr>
              </thead>
              <tbody>
                <tr>
                  <td class="mono">&gt; 0.75</td>
                  <td>interpolate 75 % → 100 %</td>
                  <td>interpolate 75 % → 100 %</td>
                </tr>
                <tr>
                  <td class="mono">0.50 – 0.75</td>
                  <td>interpolate 50 % → 75 %</td>
                  <td>interpolate 50 % → 75 %</td>
                </tr>
                <tr>
                  <td class="mono">&lt; 0.50</td>
                  <td>extrapolate below 50 %</td>
                  <td>extrapolate below 50 %</td>
                </tr>
              </tbody>
            </table>
          </div>
          <p class="small muted">
            The workbook branches on the absorbed power for the below-50 % case; the engine reproduces
            that behaviour exactly.
          </p>
        </div>
      </section>

      <section class="card">
        <div class="card-head"><h3>Load build-up</h3></div>
        <div class="card-body" style="display: flex; flex-direction: column; gap: 12px">
          <div class="formula">
            maximum normal running = continuous + max({{ fmt(settings?.intermittent_factor_pct ?? 30, 0) }} % × intermittent, largest intermittent)<br />
            peak = continuous + {{ fmt(settings?.intermittent_factor_pct ?? 30, 0) }} % × intermittent + max({{ fmt(settings?.standby_factor_pct ?? 10, 0) }} % × stand-by, largest stand-by)<br />
            total demand = peak × (1 + {{ fmt(settings?.contingency_pct ?? 20, 0) }} %)
          </div>
          <div class="table-wrap">
            <table class="grid-table">
              <thead>
                <tr>
                  <th>Duty</th>
                  <th>Definition</th>
                </tr>
              </thead>
              <tbody>
                <tr>
                  <td><span class="pill continuous"><i class="dot" />Continuous</span></td>
                  <td class="small">Required continuously during normal operation.</td>
                </tr>
                <tr>
                  <td><span class="pill intermittent"><i class="dot" />Intermittent</span></td>
                  <td class="small">Intermittent pumping, storage, loading, etc.</td>
                </tr>
                <tr>
                  <td><span class="pill standby"><i class="dot" />Stand-by</span></td>
                  <td class="small">Normally stopped, started on change-over or maintenance.</td>
                </tr>
                <tr>
                  <td><span class="pill spare"><i class="dot" />Spare</span></td>
                  <td class="small">A/B pair where only one runs at a time; not in the peak.</td>
                </tr>
              </tbody>
            </table>
          </div>
          <p class="small muted" style="line-height: 1.65">
            Consumers without a duty are excluded from the totals and flagged as findings, exactly as the
            source workbook does.
          </p>
        </div>
      </section>
    </div>

    <section class="card">
      <div class="card-head">
        <h3>Verification against the source workbook</h3>
        <div class="spacer" />
        <span class="pill neutral">tolerance 1 × 10⁻⁶ kW</span>
      </div>
      <div class="table-wrap">
        <table class="grid-table">
          <thead>
            <tr>
              <th>Scenario</th>
              <th class="num">Workbook demand (kW)</th>
              <th class="num">Engine demand (kW)</th>
              <th class="num">Difference</th>
              <th>Result</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="summary in summaries" :key="summary.scenario">
              <td>{{ summary.label }}</td>
              <td class="num dim">{{ workbook[summary.scenario] ? fmt(workbook[summary.scenario], 3) : '—' }}</td>
              <td class="num strong">{{ fmt(summary.demand.kw, 3) }}</td>
              <td class="num dim">
                {{
                  workbook[summary.scenario]
                    ? fmt(Math.abs(summary.demand.kw - workbook[summary.scenario]), 6)
                    : '—'
                }}
              </td>
              <td>
                <span
                  class="pill"
                  :class="
                    workbook[summary.scenario] &&
                    Math.abs(summary.demand.kw - workbook[summary.scenario]) < 1e-6
                      ? 'continuous'
                      : 'neutral'
                  "
                >
                  {{ workbook[summary.scenario] ? 'reproduced' : 'not tabulated' }}
                </span>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
      <div class="card-body small muted">
        The same numbers are asserted by the Rust integration tests
        (<span class="mono">backend/tests/workbook.rs</span>), so a regression in the calculation chain
        fails the build.
      </div>
    </section>

    <section class="card">
      <div class="card-head"><h3>Data sources</h3></div>
      <div class="card-body grid cols-3" style="gap: 16px">
        <div>
          <div class="eyebrow" style="margin-bottom: 6px">Overall load list</div>
          <p class="small muted" style="line-height: 1.6">
            Consumers, duties, scenarios and power data extracted from the workbook
            <span class="mono">LOAD LIST.xlsx</span> (sheet “Overall Load List”).
          </p>
        </div>
        <div>
          <div class="eyebrow" style="margin-bottom: 6px">Motor data</div>
          <p class="small muted" style="line-height: 1.6">
            Sheet “Appendix-11 Motor PF and Ef”: 80 ratings from 0.11 kW to 45 MW with efficiency and
            power factor at 100 / 75 / 50 % load.
          </p>
        </div>
        <div>
          <div class="eyebrow" style="margin-bottom: 6px">Persistence</div>
          <p class="small muted" style="line-height: 1.6">
            Edits are stored server-side as JSON, so the calculated list survives restarts and can be
            exported at any time.
          </p>
        </div>
      </div>
    </section>
  </div>
</template>
