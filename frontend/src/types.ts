export type ScenarioKey =
  | 'normal'
  | 'offloading'
  | 'maintenance'
  | 'essential'
  | 'emergency'
  | 'black_start'

export type DutyKey = 'continuous' | 'intermittent' | 'standby' | 'spare'
export type ClassKey = 'motor' | 'feeder' | 'other'

export interface LoadItem {
  id: string
  line_no: number | null
  revision: string | null
  status: string | null
  unit: string | null
  area: string | null
  system: string | null
  tag: string | null
  description: string | null
  equipment_class: ClassKey
  feeder_type: string | null
  duty: DutyKey | null
  load_type: string | null
  scenarios: ScenarioKey[]
  voltage_kv: number
  phase: string | null
  substation: string | null
  switchgear: string | null
  bus: string | null
  drive_type: string | null
  breaker_a: number | null
  absorbed_kw: number
  rated_kw: number
  remark: string | null
}

export interface ItemWarning {
  code: string
  message: string
}

export interface Derived {
  load_factor: number
  efficiency: number
  power_factor: number
  basis: string
  flc_a: number
  consumed_kw: number
  consumed_kvar: number
  consumed_kva: number
  starting_current_a: number | null
  starting_kva: number | null
  warnings: ItemWarning[]
}

export interface ItemView extends LoadItem {
  derived: Derived
}

export interface Totals {
  kw: number
  kvar: number
  kva: number
}

export interface LargestLoads {
  continuous: Totals
  intermittent: Totals
  standby: Totals
  spare: Totals
}

export interface BreakdownRow {
  key: string
  items: number
  continuous_kw: number
  intermittent_kw: number
  standby_kw: number
  spare_kw: number
  connected_kw: number
  connected_kvar: number
}

export interface GeneratorOption {
  label: string
  units: number
  running: number
  unit_kw: number
  unit_kva: number
}

export interface ScenarioSummary {
  scenario: ScenarioKey
  label: string
  continuous: Totals
  intermittent: Totals
  standby: Totals
  spare: Totals
  largest: LargestLoads
  max_normal_running: Totals
  peak: Totals
  demand: Totals
  contingency_pct: number
  peak_power_factor: number
  active_items: number
  by_voltage: BreakdownRow[]
  by_area: BreakdownRow[]
  generator_sizing: GeneratorOption[]
}

export interface MotorRef {
  rated_kw: number
  starting_ratio: number | null
  eff_full: number | null
  eff_75: number | null
  eff_50: number | null
  pf_full: number | null
  pf_75: number | null
  pf_50: number | null
}

export interface CalcSettings {
  contingency_pct: number
  intermittent_factor_pct: number
  standby_factor_pct: number
}

export interface Options {
  areas: string[]
  systems: string[]
  voltages: number[]
  statuses: string[]
  feeder_types: string[]
  load_types: string[]
  switchgears: string[]
}

export interface AuthStatus {
  authenticated: boolean
  user: string | null
  cap_endpoint: string | null
  cap_required: boolean
}

export interface Stats {
  items: number
  items_with_power: number
  warnings: number
  motor_table_rows: number
}

export interface AppState {
  project: { name: string; document: string; revision: string }
  settings: CalcSettings
  items: ItemView[]
  summaries: ScenarioSummary[]
  motor_table: MotorRef[]
  options: Options
  stats: Stats
}

export const SCENARIOS: { key: ScenarioKey; label: string; short: string }[] = [
  { key: 'normal', label: 'Normal / Holding', short: 'Normal' },
  { key: 'offloading', label: 'Offloading', short: 'Offloading' },
  { key: 'maintenance', label: 'Maintenance', short: 'Maintenance' },
  { key: 'essential', label: 'Essential', short: 'Essential' },
  { key: 'emergency', label: 'Emergency', short: 'Emergency' },
  { key: 'black_start', label: 'Black Start', short: 'Black start' },
]

export const DUTIES: { key: DutyKey; label: string; short: string }[] = [
  { key: 'continuous', label: 'Continuous', short: 'C' },
  { key: 'intermittent', label: 'Intermittent', short: 'I' },
  { key: 'standby', label: 'Stand-by', short: 'SB' },
  { key: 'spare', label: 'Spare', short: 'SP' },
]

export const CLASSES: { key: ClassKey; label: string }[] = [
  { key: 'motor', label: 'Motor' },
  { key: 'feeder', label: 'Feeder' },
  { key: 'other', label: 'Other' },
]

export const DUTY_COLORS: Record<DutyKey, string> = {
  continuous: '#ff7a18',
  intermittent: '#eab308',
  standby: '#7c3aed',
  spare: '#94a3b8',
}

export function scenarioLabel(key: ScenarioKey): string {
  return SCENARIOS.find((entry) => entry.key === key)?.label ?? key
}

export function dutyLabel(key: DutyKey | null): string {
  if (!key) return 'Unassigned'
  return DUTIES.find((entry) => entry.key === key)?.label ?? key
}

/* ------------------------------------------------------------ cable sizing */
export type Phases = 'single' | 'three'
export type CableCores = 'two_core_earth' | 'three_core_earth' | 'four_core_earth'

export const CABLE_CORES: { key: CableCores; label: string }[] = [
  { key: 'two_core_earth', label: '2C+E' },
  { key: 'three_core_earth', label: '3C+E' },
  { key: 'four_core_earth', label: '4C+E' },
]

export const PHASES: { key: Phases; label: string }[] = [
  { key: 'single', label: '1-phase' },
  { key: 'three', label: '3-phase' },
]

export interface CableReference {
  cores: CableCores
  size_mm2: number
  ampacity_a: number
  resistance_ohm_km: number
  reactance_ohm_km: number
  source: string | null
}

export interface CableCircuit {
  id: string
  si_no: number | null
  tag: string | null
  group: string | null
  voltage_grade: string | null
  cores: CableCores
  size_mm2: number
  cable_type: string | null
  length_m: number
  from_description: string | null
  from_tag: string | null
  from_location: string | null
  to_description: string | null
  to_tag: string | null
  to_location: string | null
  building: string | null
  feeder_load: string | null
  equipment_kw: number | null
  load_kw: number
  voltage_v: number
  phases: Phases
  power_factor: number
  protection_rating_a: number
  breaker_rating_a: number
  breaker_type: string | null
  runs_selected: number
  grouping_factor: number
  temperature_factor: number
  allowable_vd_pct: number | null
  remark: string | null
}

export interface CableChecks {
  ampacity: boolean
  protection: boolean
  voltage_drop: boolean | null
  max_length: boolean | null
}

export interface CableWarning {
  code: string
  message: string
}

export interface CableDerived {
  iflc_a: number
  ampacity_a: number
  derating: number
  iz_a: number
  runs_required: number
  vd_v: number
  vd_pct: number
  vd_last_pct: number | null
  chain_length_m: number | null
  max_length_m: number | null
  voltage_at_end_v: number
  checks: CableChecks
  acceptable: boolean
  warnings: CableWarning[]
}

export interface CableRow extends CableCircuit {
  derived: CableDerived
}

export interface CableStats {
  circuits: number
  acceptable: number
  failed: number
  groups: number
  total_length_m: number
  reference_rows: number
}

export interface CableState {
  project: { name: string; document: string; revision: string }
  reference: CableReference[]
  circuits: CableRow[]
  stats: CableStats
}

/* ----------------------------------------------------- cathodic protection */
export type CathodicStructure = 'tank' | 'vessel' | 'piping' | 'pipeline'
export type CathodicOrientation = 'vertical' | 'horizontal' | 'rectangular'
export type CathodicSurface = 'internal' | 'external'
export type CathodicMethod = 'sacp' | 'iccp'

export const CATHODIC_STRUCTURES: { key: CathodicStructure; label: string }[] = [
  { key: 'tank', label: 'Tank' },
  { key: 'vessel', label: 'Vessel' },
  { key: 'piping', label: 'Piping' },
  { key: 'pipeline', label: 'Pipeline' },
]

export const CATHODIC_ORIENTATIONS: { key: CathodicOrientation; label: string }[] = [
  { key: 'vertical', label: 'Vertical' },
  { key: 'horizontal', label: 'Horizontal' },
  { key: 'rectangular', label: 'Rectangular' },
]

export const CATHODIC_SURFACES: { key: CathodicSurface; label: string }[] = [
  { key: 'internal', label: 'Internal' },
  { key: 'external', label: 'External' },
]

export const CATHODIC_METHODS: { key: CathodicMethod; label: string }[] = [
  { key: 'sacp', label: 'SACP' },
  { key: 'iccp', label: 'ICCP' },
]

export interface RectifierInputs {
  anode_output_a: number
  groundbed_resistance_ohm: number
  cable_resistance_ohm: number
  back_emf_v: number
  margin_pct: number
}

export interface CathodicItem {
  id: string
  tag: string | null
  description: string | null
  source_sheet: string | null
  structure: CathodicStructure
  orientation: CathodicOrientation
  surface: CathodicSurface
  method: CathodicMethod
  design_life_years: number
  coating_breakdown: number
  apply_coating_breakdown: boolean
  current_density_a_m2: number
  temp_coefficient: number
  design_temp_c: number
  operating_temp_c: number
  capacity_temp_c: number | null
  capacity_ah_kg: number
  utilization: number
  safety_factor: number
  diameter_m: number
  length_m: number
  width_m: number
  liquid_height_m: number
  discrepancy_pct: number
  current_density_override: number | null
  anode_type: string | null
  anode_weight_kg: number
  anode_output_a: number | null
  anodes_selected: number
  rectifier: RectifierInputs | null
  remark: string | null
}

export interface CathodicChecks {
  capacity: boolean
  current_density: boolean
  anode_count: boolean
  design_life: boolean
}

export interface CathodicWarning {
  code: string
  message: string
}

export interface CathodicDerived {
  current_density_a_m2: number
  capacity_at_temp_ah_kg: number
  area_shell_m2: number
  area_ends_m2: number
  wetted_area_m2: number
  discrepancy_area_m2: number
  exposed_area_m2: number
  protection_current_a: number
  anode_mass_kg: number | null
  anodes_by_mass: number | null
  anodes_by_current: number | null
  anodes_required: number
  anodes_selected: number
  life_years: number | null
  rectifier_current_a: number | null
  rectifier_voltage_v: number | null
  rectifier_kva: number | null
  checks: CathodicChecks
  acceptable: boolean
  warnings: CathodicWarning[]
}

export interface CathodicRow extends CathodicItem {
  derived: CathodicDerived
}

export interface CathodicStats {
  items: number
  sacp: number
  iccp: number
  acceptable: number
  failed: number
  warnings: number
  total_protection_current_a: number
  total_anode_mass_kg: number
  total_anodes: number
}

export interface CathodicState {
  project: { name: string; document: string; revision: string }
  items: CathodicRow[]
  stats: CathodicStats
}
