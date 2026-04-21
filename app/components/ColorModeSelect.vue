<template>
  <USelect
    :model-value="modelValue"
    :items="items"
    value-key="value"
    label-key="label"
    :leading-icon="selectedItem.icon"
    :ui="{ base: 'w-36' }"
    @update:model-value="value => emit('update:modelValue', value)"
  />
</template>

<script setup lang="ts">
type ThemeMode = 'dark' | 'light'

const props = defineProps<{
  modelValue: ThemeMode
}>()

const emit = defineEmits<{
  'update:modelValue': [value: ThemeMode]
}>()

const items: Array<{ label: string, value: ThemeMode, icon: string }> = [
  { label: '深色', value: 'dark', icon: 'i-lucide-moon-star' },
  { label: '浅色', value: 'light', icon: 'i-lucide-sun-medium' },
]

const selectedItem = computed(() => {
  return items.find(item => item.value === props.modelValue) ?? items[0]
})
</script>
