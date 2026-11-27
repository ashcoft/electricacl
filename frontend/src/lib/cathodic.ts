import type {
  CathodicChecks,
  CathodicDerived,
  CathodicItem,
  CathodicMethod,
  CathodicOrientation,
  CathodicStructure,
  CathodicSurface,
  CathodicWarning,
} from '../types'

const PI = Math.PI
const CAPACITY_SLOPE = 27
const HOURS_PER_YEAR = 8760

function roundHalfUp(value: number, digits: number): number {
  const scale = 10 ** digits
  return Math.floor(value * scale + 0.5) / scale
}

function horizontalFill(item: CathodicItem): number {
  if (item.surface === 'external') return 1
  if (item.diameter_m <= 0) return 0
  return Math.min(1, Math.max(0, item.liquid_height_m / item.diameter_m))
}

function verticalEndArea(item: CathodicItem, diameter: number): number {
  const area = (PI * diameter * diameter) / 4
  return item.structure === 'vessel' ? 1.5 * area : area
}

function wallHeight(item: CathodicItem): number {
  if (item.surface !== 'external') return item.liquid_height_m
  if (item.height_m > 0) return item.height_m
  return item.length_m
}

function geometry(item: CathodicItem): [number, number, number] {
  const diameter = item.diameter_m
  const height = wallHeight(item)
  if (item.structure === 'piping' || item.structure === 'pipeline') {
    const shell = PI * diameter * item.length_m
    return [shell, 0, shell]
  }
  if (item.orientation === 'rectangular') {
    const shell = 2 * (item.length_m + item.width_m) * height
    const ends = 2 * item.length_m * item.width_m
    return [shell, ends, shell + ends]
  }
  if (item.orientation === 'vertical') {
    const shell = PI * diameter * height
    const ends = verticalEndArea(item, diameter)
    return [shell, ends, shell + ends]
  }
  const fill = horizontalFill(item)
  const shell = PI * diameter * item.length_m * fill
  const ends = ((3 * PI * diameter * diameter) / 4) * fill
  return [shell, ends, shell + ends]
}

function currentDensity(item: CathodicItem): number {
  if (item.current_density_override !== null) return item.current_density_override
  const corrected = item.current_density_a_m2 + (item.design_temp_c - 25) * item.temp_coefficient
  if (!item.apply_coating_breakdown) return corrected
  return corrected * item.coating_breakdown
}

function capacityTemperature(item: CathodicItem): number {
  if (typeof item.capacity_temp_c === 'number' && Number.isFinite(item.capacity_temp_c)) {
    return item.capacity_temp_c
  }
  return item.operating_temp_c
}

function capacityAtTemperature(item: CathodicItem): number {
  return item.capacity_ah_kg - CAPACITY_SLOPE * (capacityTemperature(item) - 20)
}

function anodeMassFor(item: CathodicItem, protectionCurrent: number, capacity: number): number | null {
  if (capacity <= 0 || item.utilization <= 0) return null
  return (
    (protectionCurrent * item.design_life_years * HOURS_PER_YEAR) /
    capacity /
    item.utilization *
    item.safety_factor
  )
}

function countByMass(mass: number | null, weight: number): number | null {
  if (mass === null) return null
  if (weight <= 0) return 0
  return Math.max(0, Math.ceil(mass / weight - 1e-9))
}

function countByCurrent(protectionCurrent: number, output: number | null): number | null {
  if (output === null || output <= 0) return null
  return Math.max(0, Math.ceil(protectionCurrent / output - 1e-9))
}

function actualLife(item: CathodicItem, protectionCurrent: number, capacity: number): number | null {
  if (
    item.anodes_selected <= 0 ||
    item.anode_weight_kg <= 0 ||
    item.utilization <= 0 ||
    protectionCurrent <= 0 ||
    capacity <= 0
  ) {
    return null
  }
  return (
    (item.anodes_selected * item.anode_weight_kg * capacity * item.utilization) /
    (protectionCurrent * HOURS_PER_YEAR * item.safety_factor)
  )
}

function sacpWarnings(
  item: CathodicItem,
  mass: number | null,
  byMass: number | null,
  life: number | null,
): CathodicWarning[] {
  const warnings: CathodicWarning[] = []
  if (mass !== null && item.anode_weight_kg <= 0) {
    warnings.push({
      code: 'no_anode_weight',
      message: 'Anode unit weight is zero - the number of anodes cannot be sized',
    })
  }
  if (byMass !== null && item.anodes_selected < byMass) {
    warnings.push({
      code: 'anodes_short',
      message: `${item.anodes_selected} anodes selected, ${byMass} required by the mass balance`,
    })
  }
  if (life !== null && life < item.design_life_years) {
    warnings.push({
      code: 'life_short',
      message: `Selected anodes give ${life.toFixed(1)} years, below the ${item.design_life_years.toFixed(1)}-year design life`,
    })
  }
  return warnings
}

interface IccpSizing {
  current: number
  voltage: number
  kva: number
  required: number
  warnings: CathodicWarning[]
}

function iccpSizing(item: CathodicItem, protectionCurrent: number): IccpSizing {
  const rectifier = item.rectifier ?? {
    anode_output_a: 5,
    groundbed_resistance_ohm: 1,
    cable_resistance_ohm: 0.2,
    back_emf_v: 2,
    margin_pct: 20,
  }
  const voltage =
    (protectionCurrent * (rectifier.groundbed_resistance_ohm + rectifier.cable_resistance_ohm) +
      rectifier.back_emf_v) *
    (1 + rectifier.margin_pct / 100)
  const required = countByCurrent(protectionCurrent, rectifier.anode_output_a) ?? 0
  const warnings: CathodicWarning[] = []
  if (rectifier.anode_output_a <= 0) {
    warnings.push({
      code: 'no_groundbed_output',
      message: 'Groundbed anode output is zero - the anode count cannot be sized',
    })
  } else if (item.anodes_selected < required) {
    warnings.push({
      code: 'anodes_short',
      message: `${item.anodes_selected} groundbed anodes selected, ${required} required for ${protectionCurrent.toFixed(1)} A`,
    })
  }
  return {
    current: protectionCurrent,
    voltage,
    kva: (voltage * protectionCurrent) / 1000,
    required,
    warnings,
  }
}

function designLifeOk(method: CathodicMethod, life: number | null, designLife: number): boolean {
  if (method === 'iccp') return true
  if (life === null) return false
  return life >= designLife
}

/**
 * Client-side mirror of the Rust cathodic engine, used for instant feedback
 * while editing. The server result always wins after saving.
 */
export function computeCathodicDerived(item: CathodicItem): CathodicDerived {
  const warnings: CathodicWarning[] = []
  const capacityTemp = capacityTemperature(item)
  const capacityAtTemp = capacityAtTemperature(item)
  const density = currentDensity(item)
  const [areaShell, areaEnds, wetted] = geometry(item)
  const discrepancyArea = roundHalfUp((wetted * item.discrepancy_pct) / 100, 2)
  const exposed = wetted + discrepancyArea
  const protectionCurrent = exposed * density

  if (capacityAtTemp <= 0) {
    warnings.push({
      code: 'capacity_not_positive',
      message: `Anode capacity at ${capacityTemp.toFixed(1)} °C is ${capacityAtTemp.toFixed(1)} Ah/kg - the mass balance cannot be closed`,
    })
  }
  if (density <= 0) {
    warnings.push({
      code: 'no_current_density',
      message: 'Design current density is zero - check the coating breakdown and temperature',
    })
  }
  if (exposed <= 0) {
    warnings.push({ code: 'no_area', message: 'Exposed area is zero - check the geometry and liquid height' })
  }

  let anodeMass: number | null = null
  let anodesByMass: number | null = null
  let anodesByCurrent: number | null = null
  let lifeYears: number | null = null
  let rectifierCurrent: number | null = null
  let rectifierVoltage: number | null = null
  let rectifierKva: number | null = null
  let required = 0
  let sizingAvailable = true

  if (item.method === 'sacp') {
    sizingAvailable = item.anode_weight_kg > 0
    anodeMass = anodeMassFor(item, protectionCurrent, capacityAtTemp)
    anodesByMass = countByMass(anodeMass, item.anode_weight_kg)
    anodesByCurrent = countByCurrent(protectionCurrent, item.anode_output_a)
    required = Math.max(anodesByMass ?? 0, anodesByCurrent ?? 0)
    lifeYears = actualLife(item, protectionCurrent, capacityAtTemp)
    warnings.push(...sacpWarnings(item, anodeMass, anodesByMass, lifeYears))
  } else {
    const iccp = iccpSizing(item, protectionCurrent)
    sizingAvailable = (item.rectifier?.anode_output_a ?? 5) > 0
    rectifierCurrent = iccp.current
    rectifierVoltage = iccp.voltage
    rectifierKva = iccp.kva
    required = iccp.required
    warnings.push(...iccp.warnings)
  }

  const checks: CathodicChecks = {
    capacity: capacityAtTemp > 0,
    current_density: density > 0,
    anode_count: sizingAvailable && item.anodes_selected >= required,
    design_life: designLifeOk(item.method, lifeYears, item.design_life_years),
  }

  return {
    current_density_a_m2: density,
    capacity_at_temp_ah_kg: capacityAtTemp,
    area_shell_m2: areaShell,
    area_ends_m2: areaEnds,
    wetted_area_m2: wetted,
    discrepancy_area_m2: discrepancyArea,
    exposed_area_m2: exposed,
    protection_current_a: protectionCurrent,
    anode_mass_kg: anodeMass,
    anodes_by_mass: anodesByMass,
    anodes_by_current: anodesByCurrent,
    anodes_required: required,
    anodes_selected: item.anodes_selected,
    life_years: lifeYears,
    rectifier_current_a: rectifierCurrent,
    rectifier_voltage_v: rectifierVoltage,
    rectifier_kva: rectifierKva,
    checks,
    acceptable:
      checks.capacity && checks.current_density && checks.anode_count && checks.design_life && exposed > 0,
    warnings,
  }
}

export function blankCathodicItem(): CathodicItem {
  return {
    id: '',
    tag: null,
    description: null,
    source_sheet: null,
    structure: 'tank',
    orientation: 'vertical',
    surface: 'internal',
    method: 'sacp',
    design_life_years: 20,
    coating_breakdown: 0.1,
    apply_coating_breakdown: true,
    current_density_a_m2: 0.07,
    temp_coefficient: 0.001,
    design_temp_c: 25,
    operating_temp_c: 25,
    capacity_temp_c: null,
    capacity_ah_kg: 2500,
    utilization: 0.85,
    safety_factor: 1,
    diameter_m: 10,
    length_m: 10,
    width_m: 10,
    height_m: 0,
    liquid_height_m: 5,
    discrepancy_pct: 10,
    current_density_override: null,
    anode_type: 'ICS 40 AWB',
    anode_weight_kg: 4,
    anode_output_a: 0.43,
    anodes_selected: 1,
    rectifier: null,
    remark: null,
  }
}

export function structureLabel(structure: CathodicStructure): string {
  return { tank: 'Tank', vessel: 'Vessel', piping: 'Piping', pipeline: 'Pipeline' }[structure]
}

export function orientationLabel(orientation: CathodicOrientation): string {
  return { vertical: 'Vertical', horizontal: 'Horizontal', rectangular: 'Rectangular' }[orientation]
}

export function surfaceLabel(surface: CathodicSurface): string {
  return surface === 'internal' ? 'Internal' : 'External'
}

export function methodLabel(method: CathodicMethod): string {
  return method === 'sacp' ? 'SACP' : 'ICCP'
}

export function scopeLabel(item: CathodicItem): string {
  if (item.structure === 'piping' || item.structure === 'pipeline') {
    return `${structureLabel(item.structure)} · ${surfaceLabel(item.surface)}`
  }
  return `${structureLabel(item.structure)} · ${orientationLabel(item.orientation)} · ${surfaceLabel(item.surface)}`
}
