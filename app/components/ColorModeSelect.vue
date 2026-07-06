<template>
  <USelect
    :model-value="modelValue"
    :items="items"
    value-key="value"
    label-key="label"
    :leading-icon="selectedItem?.icon"
    :ui="{ base: 'w-36' }"
    @update:model-value="value => emit('update:modelValue', value)"
  />
</template>

<script setup lang="ts">
type ThemeMode = 'dark' | 'light' | 'system'

const props = defineProps<{
  modelValue: ThemeMode
}>()

const emit = defineEmits<{
  'update:modelValue': [value: ThemeMode]
}>()

const { t } = useI18n()

const items = computed<Array<{ label: string, value: ThemeMode, icon: string }>>(() => [
  { label: t('theme.dark'), value: 'dark', icon: 'i-lucide-moon-star' },
  { label: t('theme.light'), value: 'light', icon: 'i-lucide-sun-medium' },
  { label: t('theme.system'), value: 'system', icon: 'i-lucide-computer' },
])

const selectedItem = computed(() => {
  return items.value.find(item => item.value === props.modelValue) ?? items.value[0]!
})
</script>
