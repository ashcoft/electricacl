import type { Derived, ItemWarning, LoadItem, MotorRef } from '../types'

const SQRT3 = 1.732

/**
 * Client-side mirror of the Rust calculation, used to give instant feedback
 * while the user edits an item. The server result always wins after saving.
 */
export function computeDerived(item: LoadItem, table: MotorRef[]): Derived {
  const rated = item.rated_kw
  const absorbed = item.absorbed_kw
  const loadFactor = rated > 0 ? absorbed / rated : 0

  let efficiency = 0.95
  let powerFactor = 0.9
  let basis = 'feeder'
  let startingRatio: number | null = null

  if (item.equipment_class === 'other') {
    efficiency = 1
    powerFactor = 1
    basis = 'other'
  } else if (item.equipment_class === 'motor') {
    const row = table.find((entry) => Math.abs(entry.rated_kw - rated) < 1e-9)
    startingRatio = row?.starting_ratio ?? null
    if (
      !row ||
      row.eff_full === null ||
      row.eff_75 === null ||
      row.eff_50 === null ||
      row.pf_full === null ||
      row.pf_75 === null ||
      row.pf_50 === null
    ) {
      basis = 'fallback'
    } else {
      basis = 'motor_table'
      if (rated <= 0) {
        efficiency = row.eff_full
        powerFactor = row.pf_full
      } else if (loadFactor > 0.75) {
        const t = (loadFactor - 0.75) / 0.25
        efficiency = row.eff_75 + (row.eff_full - row.eff_75) * t
        powerFactor = row.pf_75 + (row.pf_full - row.pf_75) * t
      } else if (absorbed > 0.5) {
        const t = (loadFactor - 0.5) / 0.25
        efficiency = row.eff_50 + (row.eff_75 - row.eff_50) * t
        powerFactor = row.pf_50 + (row.pf_75 - row.pf_50) * t
      } else {
        const t = (0.5 - loadFactor) / 0.25
        efficiency = row.eff_50 + (row.eff_75 - row.eff_50) * t
        powerFactor = row.pf_50 - (row.pf_75 - row.pf_50) * t
      }
      efficiency = Math.min(1, Math.max(0.01, efficiency))
      powerFactor = Math.min(1, Math.max(0.01, powerFactor))
    }
  }

  const flc =
    item.voltage_kv > 0 && efficiency > 0 && powerFactor > 0
      ? rated / (SQRT3 * item.voltage_kv * efficiency * powerFactor)
      : 0
  const consumedKw = efficiency > 0 ? absorbed / efficiency : 0
  const consumedKvar =
    consumedKw * (Math.sqrt(Math.max(0, 1 - powerFactor * powerFactor)) / powerFactor)

  const warnings: ItemWarning[] = []
  if (!item.duty) {
    warnings.push({
      code: 'no_duty',
      message: 'No service duty assigned - excluded from every load column',
    })
  }
  if (!item.scenarios.length) {
    warnings.push({
      code: 'no_scenario',
      message: 'No operating scenario selected - excluded from all scenarios',
    })
  }
  if (basis === 'fallback') {
    warnings.push({
      code: 'motor_not_in_table',
      message: `Motor rating ${rated} kW is not in the reference table - feeder defaults (0.95 / 0.90) applied`,
    })
  }
  if (loadFactor > 1.02) {
    warnings.push({
      code: 'load_factor_high',
      message: `Absorbed power exceeds nameplate rating (load factor ${Math.round(loadFactor * 100)} %)`,
    })
  }
  if (absorbed <= 0 && rated <= 0) {
    warnings.push({ code: 'zero_power', message: 'No absorbed or rated power entered' })
  }

  return {
    load_factor: loadFactor,
    efficiency,
    power_factor: powerFactor,
    basis,
    flc_a: flc,
    consumed_kw: consumedKw,
    consumed_kvar: consumedKvar,
    consumed_kva: Math.hypot(consumedKw, consumedKvar),
    starting_current_a: startingRatio !== null ? flc * startingRatio : null,
    starting_kva: startingRatio !== null ? Math.sqrt(3) * item.voltage_kv * flc * startingRatio : null,
    warnings,
  }
}

export function blankItem(): LoadItem {
  return {
    id: '',
    line_no: null,
    revision: null,
    status: 'NEW',
    unit: null,
    area: 'T',
    system: null,
    tag: null,
    description: null,
    equipment_class: 'feeder',
    feeder_type: null,
    duty: 'continuous',
    load_type: null,
    scenarios: ['normal'],
    voltage_kv: 0.69,
    phase: '3PH',
    substation: null,
    switchgear: null,
    bus: null,
    drive_type: null,
    breaker_a: null,
    absorbed_kw: 0,
    rated_kw: 0,
    remark: null,
  }
}
