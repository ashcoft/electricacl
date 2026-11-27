import type {
  CableChecks,
  CableCircuit,
  CableDerived,
  CableReference,
  CableWarning,
  CableCores,
} from '../types'

const SQRT3 = 1.732

/**
 * Client-side mirror of the Rust cable engine, used for instant feedback while
 * editing. The server result always wins after saving.
 */
export function computeCableDerived(
  circuit: CableCircuit,
  reference: CableReference[],
): CableDerived {
  const warnings: CableWarning[] = []
  const pf = Math.min(1, Math.max(0.01, circuit.power_factor))
  const sinPhi = Math.sqrt(Math.max(0, 1 - pf * pf))
  const kCurrent = circuit.phases === 'three' ? SQRT3 : 1
  const kDrop = circuit.phases === 'three' ? SQRT3 : 2
  const runs = Math.max(1, circuit.runs_selected)

  const iflc =
    circuit.voltage_v > 0 ? (circuit.load_kw * 1000) / (kCurrent * circuit.voltage_v * pf) : 0

  const row = reference.find(
    (entry) => entry.cores === circuit.cores && Math.abs(entry.size_mm2 - circuit.size_mm2) < 1e-9,
  )
  if (!row) {
    warnings.push({
      code: 'cable_not_in_table',
      message: `${coresLabel(circuit.cores)} x ${circuit.size_mm2} mm2 is not in the cable reference table - ampacity and impedance unknown`,
    })
  }
  const ampacity = row?.ampacity_a ?? 0
  const resistance = row?.resistance_ohm_km ?? 0
  const reactance = row?.reactance_ohm_km ?? 0

  const derating = circuit.grouping_factor * circuit.temperature_factor
  const iz = ampacity * derating
  const runsRequired = iz > 0 ? iflc / iz : 0
  const impedance = resistance * pf + reactance * sinPhi
  const vd = (kDrop * iflc * (circuit.length_m / 1000) * impedance) / runs
  const vdPct = circuit.voltage_v > 0 ? (vd / circuit.voltage_v) * 100 : 0
  const denominator = (kDrop * iflc * (1 / 1000) * impedance) / runs
  const maxLength =
    circuit.allowable_vd_pct !== null && denominator > 0
      ? ((circuit.allowable_vd_pct / 100) * circuit.voltage_v) / denominator
      : null

  if (circuit.load_kw <= 0) {
    warnings.push({ code: 'no_load', message: 'No load entered - current and drop are zero' })
  }
  if (iz > 0 && iflc > iz) {
    warnings.push({
      code: 'ampacity_exceeded',
      message: `Design current ${iflc.toFixed(1)} A exceeds the derated capacity ${iz.toFixed(1)} A`,
    })
  }
  if (iz > 0 && circuit.protection_rating_a > 0 && iz <= circuit.protection_rating_a) {
    warnings.push({
      code: 'protection_not_coordinated',
      message: `Derated capacity ${iz.toFixed(1)} A does not exceed the protection setting ${circuit.protection_rating_a.toFixed(1)} A`,
    })
  }

  const checks: CableChecks = {
    ampacity: iz > 0 && iz >= iflc,
    protection: iz > 0 && iz > circuit.protection_rating_a,
    voltage_drop: null,
    max_length: null,
  }

  return {
    iflc_a: iflc,
    ampacity_a: ampacity,
    derating,
    iz_a: iz,
    runs_required: runsRequired,
    vd_v: vd,
    vd_pct: vdPct,
    vd_last_pct: null,
    chain_length_m: null,
    max_length_m: maxLength,
    voltage_at_end_v: circuit.voltage_v - vd,
    checks,
    acceptable: false,
    warnings,
  }
}

export function coresLabel(cores: CableCores): string {
  switch (cores) {
    case 'three_core_earth':
      return '3C+E'
    case 'four_core_earth':
      return '4C+E'
    default:
      return '2C+E'
  }
}

export function cableLabel(cores: CableCores, sizeMm2: number): string {
  return `${coresLabel(cores)} x ${sizeMm2} mm2`
}

export function blankCableCircuit(reference: CableReference[]): CableCircuit {
  const first = reference[0]
  return {
    id: '',
    si_no: null,
    tag: null,
    group: null,
    voltage_grade: '0.6/1kV',
    cores: first?.cores ?? 'two_core_earth',
    size_mm2: first?.size_mm2 ?? 2.5,
    cable_type: 'CU/XLPE/LSZH - FRT',
    length_m: 20,
    from_description: null,
    from_tag: null,
    from_location: null,
    to_description: null,
    to_tag: null,
    to_location: null,
    building: null,
    feeder_load: 'LIGHTING',
    equipment_kw: null,
    load_kw: 0,
    voltage_v: 230,
    phases: 'single',
    power_factor: 0.9,
    protection_rating_a: 10,
    breaker_rating_a: 10,
    breaker_type: 'MCB',
    runs_selected: 1,
    grouping_factor: 0.66,
    temperature_factor: 1,
    allowable_vd_pct: 3,
    remark: null,
  }
}
