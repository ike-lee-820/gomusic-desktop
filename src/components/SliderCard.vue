<script setup lang="ts">
import { computed } from 'vue'

const props = withDefaults(defineProps<{
  title: string; modelValue: number; min?: number; max?: number; step?: number; unit?: string
}>(), { min: 0, max: 100, step: 1, unit: '%' })
const emit = defineEmits<{ (e: 'update:modelValue', v: number): void }>()

const value = computed({
  get: () => props.modelValue,
  set: (v: number) => emit('update:modelValue', v),
})

// 滑块卡片本身消费纵向 wheel，父容器不拦截（对应手机端 nestedScroll）。
function onWheel(e: WheelEvent) { e.stopPropagation() }
</script>

<template>
  <div class="go-card slider-card">
    <div class="go-group-title">{{ title }}</div>
    <div class="line">
      <span class="min">{{ min }}</span>
      <input
        v-model.number="value" type="range" :min="min" :max="max" :step="step"
        @wheel="onWheel" @pointerdown.stop
      />
      <span class="val tabular">{{ value }}{{ unit }}</span>
    </div>
    <div v-if="$slots.default" class="extra"><slot /></div>
  </div>
</template>

<style scoped>
.slider-card { overflow: hidden; }
.line { display: flex; align-items: center; gap: 12px; }
.min { font-size: 11px; color: var(--go-text-3); min-width: 32px; }
.val { font-size: 13px; color: var(--go-text); min-width: 56px; text-align: right; }
input[type='range'] { flex: 1; height: 4px; accent-color: var(--go-accent); cursor: pointer; }
.extra { margin-top: 12px; }
</style>
