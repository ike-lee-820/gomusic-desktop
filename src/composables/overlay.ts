/** 轻量全局浮层：Toast 2.5s 自动消失，Dialog 二次确认。 */
type ToastFn = (text: string, danger?: boolean) => void
type ConfirmFn = (title: string, text: string, onOk: () => void) => void

let toastCb: ToastFn = () => {}
let confirmCb: ConfirmFn = () => {}

export const toast = {
  bind(fn: (text: string, danger: boolean) => void) {
    toastCb = (text, danger = false) => fn(text, danger)
  },
  show: (text: string, danger = false) => toastCb(text, danger),
  error: (text: string) => toastCb(text, true),
}

export const confirm = {
  bind(fn: ConfirmFn) { confirmCb = fn },
  ask: (title: string, text: string, onOk: () => void) => confirmCb(title, text, onOk),
}
