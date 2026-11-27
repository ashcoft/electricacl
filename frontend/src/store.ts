import { computed, reactive } from 'vue'
import { api } from './lib/api'
import type {
  AppState,
  CalcSettings,
  ItemView,
  LoadItem,
  ScenarioKey,
  ScenarioSummary,
} from './types'

export type ViewKey = 'dashboard' | 'loadlist' | 'motors' | 'method'

export interface Toast {
  id: number
  kind: 'success' | 'error' | 'info'
  message: string
}

export interface Filters {
  search: string
  area: string
  voltage: string
  duty: string
  klass: string
  status: string
  warningsOnly: boolean
}

let toastSeq = 0

export const ui = reactive({
  view: 'dashboard' as ViewKey,
  scenario: 'normal' as ScenarioKey,
  loading: true,
  busy: false,
  loadError: '',
  toasts: [] as Toast[],
  drawerId: null as string | null,
  createOpen: false,
  settingsOpen: false,
  filters: {
    search: '',
    area: '',
    voltage: '',
    duty: '',
    klass: '',
    status: '',
    warningsOnly: false,
  } as Filters,
  page: 1,
  pageSize: 100,
})

export const data = reactive({
  state: null as AppState | null,
})

export const currentSummary = computed<ScenarioSummary | null>(() => {
  if (!data.state) return null
  return data.state.summaries.find((entry) => entry.scenario === ui.scenario) ?? null
})

export const drawerItem = computed<ItemView | null>(() => {
  if (!data.state || !ui.drawerId) return null
  return data.state.items.find((item) => item.id === ui.drawerId) ?? null
})

export function toast(kind: Toast['kind'], message: string) {
  const entry: Toast = { id: ++toastSeq, kind, message }
  ui.toasts.push(entry)
  window.setTimeout(() => {
    const index = ui.toasts.findIndex((candidate) => candidate.id === entry.id)
    if (index >= 0) ui.toasts.splice(index, 1)
  }, kind === 'error' ? 7000 : 3200)
}

export async function load() {
  ui.loading = true
  ui.loadError = ''
  try {
    data.state = await api.getState()
  } catch (error) {
    ui.loadError = error instanceof Error ? error.message : String(error)
  } finally {
    ui.loading = false
  }
}

async function mutate<T>(action: () => Promise<T>, successMessage?: string): Promise<T | null> {
  ui.busy = true
  try {
    const result = await action()
    data.state = await api.getState()
    if (successMessage) toast('success', successMessage)
    return result
  } catch (error) {
    toast('error', error instanceof Error ? error.message : String(error))
    return null
  } finally {
    ui.busy = false
  }
}

export function saveItem(item: LoadItem) {
  return mutate(() => api.updateItem(item), 'Load item saved')
}

export function createItem(partial: Partial<LoadItem>) {
  return mutate(() => api.createItem(partial), 'Load item added')
}

export function deleteItem(id: string) {
  return mutate(() => api.deleteItem(id), 'Load item deleted')
}

export function duplicateItem(id: string) {
  return mutate(() => api.duplicateItem(id), 'Load item duplicated')
}

export function saveSettings(settings: CalcSettings) {
  return mutate(() => api.saveSettings(settings), 'Calculation settings saved')
}

export function resetItems() {
  return mutate(() => api.resetItems(), 'Source data restored')
}

export function resetData() {
  data.state = null
  ui.loading = true
  ui.loadError = ''
  ui.drawerId = null
  ui.createOpen = false
  ui.settingsOpen = false
  ui.view = 'dashboard'
  ui.scenario = 'normal'
  resetFilters()
}

export function setScenario(scenario: ScenarioKey) {
  ui.scenario = scenario
}

export function resetFilters() {
  ui.filters = {
    search: '',
    area: '',
    voltage: '',
    duty: '',
    klass: '',
    status: '',
    warningsOnly: false,
  }
  ui.page = 1
}

/** Case-insensitive search across the fields an engineer would look at. */
export function matchesSearch(item: ItemView, needle: string): boolean {
  if (!needle) return true
  const haystack = [
    item.tag,
    item.description,
    item.system,
    item.area,
    item.switchgear,
    item.status,
    item.remark,
    item.line_no?.toString(),
  ]
    .filter(Boolean)
    .join(' ')
    .toLowerCase()
  return haystack.includes(needle.toLowerCase())
}
