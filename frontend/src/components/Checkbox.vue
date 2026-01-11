<script setup>
import { computed } from 'vue'

const props = defineProps({
  modelValue: Boolean,
  indeterminate: Boolean
})

const emit = defineEmits(['update:modelValue'])

const internalValue = computed({
  get: () => props.modelValue,
  set: (v) => emit('update:modelValue', v)
})

function toggle() {
  internalValue.value = !internalValue.value
}
</script>

<template>
  <div
    class="checkbox-wrapper"
    :class="{ checked: internalValue, indeterminate: indeterminate }"
    @click="toggle"
  >
    <svg viewBox="0 0 20 20" fill="currentColor" class="check-icon">
      <path
        v-if="indeterminate"
        fill-rule="evenodd"
        d="M4 10a1 1 0 011-1h10a1 1 0 110 2H5a1 1 0 01-1-1z"
        clip-rule="evenodd"
      />
      <path
        v-else-if="internalValue"
        fill-rule="evenodd"
        d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z"
        clip-rule="evenodd"
      />
    </svg>
  </div>
</template>

<style scoped>
.checkbox-wrapper {
  width: 20px;
  height: 20px;
  border: 2px solid var(--color-primary, #9C8671);
  border-radius: 4px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.2s ease;
  background: white;
}

.checkbox-wrapper:hover {
  border-color: var(--color-dark-brown, #6B5D52);
  background: var(--color-hover, #E8DCC4);
}

.checkbox-wrapper.checked {
  background: var(--color-primary, #9C8671);
  border-color: var(--color-primary, #9C8671);
}

.checkbox-wrapper.checked .check-icon {
  color: white;
}

.checkbox-wrapper.indeterminate {
  background: var(--color-tan, #D4C5A9);
  border-color: var(--color-tan, #D4C5A9);
}

.checkbox-wrapper.indeterminate .check-icon {
  color: white;
}

.check-icon {
  width: 14px;
  height: 14px;
  color: transparent;
  transition: color 0.2s ease;
}
</style>
