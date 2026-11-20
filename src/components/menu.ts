export interface MenuItem {
  label: string
  icon?: string
  danger?: boolean
  action: () => void
}

export interface ContextMenuHandle {
  show: (x: number, y: number, items: MenuItem[]) => void
  hide: () => void
}
