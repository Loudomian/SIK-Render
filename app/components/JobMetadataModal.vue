<template>
  <UModal
    :open="open"
    :close="false"
    title="编辑项目信息"
    :ui="{ content: 'job-modal-content metadata-modal-content' }"
    @update:open="handleOpenChange"
  >
    <template #body>
      <div class="modal-stack metadata-modal-stack">
        <div class="metadata-form">
          <UFormField label="项目名称" class="metadata-form-field">
            <UInput
              v-model.trim="draftName"
              :disabled="saving"
              size="lg"
              class="w-full"
              placeholder="例如：Shot_010_Lighting"
            />
          </UFormField>

          <UFormField label="任务备注" class="metadata-form-field">
            <UTextarea
              v-model="draftNote"
              :disabled="saving"
              :rows="4"
              autoresize
              class="w-full"
              placeholder="写一点备注，例如返修原因、交付要求、客户反馈。"
            />
          </UFormField>
        </div>

        <p v-if="errorMessage" class="form-error">{{ errorMessage }}</p>

        <div class="modal-actions">
          <UButton
            icon="i-lucide-x"
            label="取消"
            color="warning"
            variant="outline"
            :disabled="saving"
            @click="emit('update:open', false)"
          />
          <UButton
            icon="i-lucide-save"
            label="保存"
            color="primary"
            variant="solid"
            :loading="saving"
            @click="saveMetadata"
          />
        </div>
      </div>
    </template>
  </UModal>
</template>

<script setup lang="ts">
import type { RenderJob } from '~/types'

const props = defineProps<{
  open: boolean
  job: RenderJob | null
}>()

const emit = defineEmits<{
  'update:open': [value: boolean]
}>()

const jobsStore = useJobsStore()

const draftName = ref('')
const draftNote = ref('')
const saving = ref(false)
const errorMessage = ref('')

function syncDrafts() {
  draftName.value = props.job?.name ?? ''
  draftNote.value = props.job?.note ?? ''
  errorMessage.value = ''
}

watch(
  () => [props.open, props.job?.id, props.job?.name, props.job?.note] as const,
  ([open]) => {
    if (!open) return
    syncDrafts()
  },
  { immediate: true },
)

function handleOpenChange(value: boolean) {
  emit('update:open', value)
}

async function saveMetadata() {
  if (!props.job || saving.value) return

  const nextName = draftName.value.trim()
  if (!nextName) {
    errorMessage.value = '项目名称不能为空。'
    return
  }

  saving.value = true
  errorMessage.value = ''

  try {
    await jobsStore.updateJobMetadata(
      props.job.id,
      nextName,
      draftNote.value.trim() || null,
    )
    emit('update:open', false)
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : String(error)
  } finally {
    saving.value = false
  }
}
</script>
