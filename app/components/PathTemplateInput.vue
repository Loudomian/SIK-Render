<template>
  <div class="path-template-input">
    <div
      ref="editorRef"
      class="path-template-editor surface-panel"
      :class="[textareaClass, {
        'path-template-editor-empty': !hasContent,
        'path-template-editor-disabled': disabled,
        'path-template-editor-dragover': dragOver,
      }]"
      :contenteditable="disabled ? 'false' : 'true'"
      :data-placeholder="placeholder"
      spellcheck="false"
      @input="handleInput"
      @keydown="handleKeydown"
      @paste="handlePaste"
      @copy="handleCopy"
      @focus="handleFocus"
      @click="handlePointerSync"
      @keyup="handleCaretSync"
      @dragover="handleDragOver"
      @dragleave="dragOver = false"
      @drop="handleDrop"
    />

    <div v-if="showSuggestions && filteredSuggestions.length" class="path-template-suggestion-menu surface-panel">
      <button
        v-for="(variable, index) in filteredSuggestions"
        :key="`${variable.token}-${index}`"
        :ref="element => setSuggestionItemRef(element, index)"
        type="button"
        class="path-template-suggestion-item"
        :class="{ 'path-template-suggestion-item-active': index === selectedSuggestionIndex }"
        @mousedown.prevent="applySuggestion(variable.token)"
        @mouseenter="selectedSuggestionIndex = index"
      >
        <span class="path-template-suggestion-token">{{ variable.token }}</span>
        <span class="path-template-suggestion-description">{{ variable.description }}</span>
      </button>
    </div>

    <div v-if="shouldShowToolbar" class="path-template-toolbar">
      <UButton
        type="button"
        icon="i-lucide-braces"
        :label="showVariables ? t('pathTemplate.collapseVariables') : t('pathTemplate.insertVariable')"
        color="neutral"
        variant="ghost"
        size="xs"
        :disabled="disabled"
        @click="showVariables = !showVariables"
      />
    </div>

    <div v-if="shouldShowPanel" class="path-template-panel surface-panel">
      <div v-for="group in variableGroups" :key="group.label" class="path-template-group">
        <p class="path-template-group-label">{{ group.label }}</p>
        <div class="path-template-chip-row">
          <UButton
            v-for="variable in group.items"
            :key="variable.token"
            type="button"
            size="xs"
            color="neutral"
            :variant="variable.enabled ? 'outline' : 'ghost'"
            :disabled="disabled || !variable.enabled"
            :label="variable.label"
            :title="variable.description"
            draggable="true"
            @click="insertToken(variable.token)"
            @dragstart="handleVariableDragStart($event, variable)"
          />
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { ComponentPublicInstance } from 'vue'
import type { PathTemplateKind } from '~/types'

type VariableState = 'normal' | 'preview' | 'error'

type VariableItem = {
  token: string
  label: string
  enabled: boolean
  preview: string | null
  description: string
}

type ParsedSegment =
  | { type: 'text'; value: string }
  | { type: 'token'; value: string; state: VariableState; title: string }

const DEFAULT_FRAME_PADDING = 6
const MAX_FRAME_PADDING = 12
const VARIABLE_PATTERN = /\{[^{}]+\}/g
const TOKEN_PATTERN = /^\{(?:[A-Za-z][A-Za-z0-9]*|frame:\d+)\}$/
const TOKEN_DRAG_MIME = 'application/x-sik-render-path-token'

const props = defineProps<{
  modelValue: string
  kind: PathTemplateKind
  disabled?: boolean
  placeholder?: string
  textareaClass?: string
  variablePanelMode?: 'toggle' | 'hidden' | 'expanded'
  blendFileName?: string | null
  folderName?: string | null
  frameStart?: number | null
  frameEnd?: number | null
}>()

const emit = defineEmits<{
  'update:modelValue': [value: string]
  focus: []
}>()

const editorRef = ref<HTMLElement | null>(null)
const { t, locale } = useI18n()
const showVariables = ref(props.variablePanelMode === 'expanded')
const dragOver = ref(false)
const hasContent = ref(false)
const lastCaretOffset = ref<number | null>(null)
const showSuggestions = ref(false)
const suggestionQuery = ref('')
const suggestionRange = ref<{ start: number, end: number } | null>(null)
const selectedSuggestionIndex = ref(0)
const suggestionItemRefs = ref<(HTMLButtonElement | null)[]>([])
const shouldShowToolbar = computed(() => (props.variablePanelMode ?? 'toggle') === 'toggle')
const shouldShowPanel = computed(() => {
  const mode = props.variablePanelMode ?? 'toggle'
  if (mode === 'hidden') return false
  if (mode === 'expanded') return true
  return showVariables.value
})

function setSuggestionItemRef(element: Element | ComponentPublicInstance | null, index: number) {
  suggestionItemRefs.value[index] = element instanceof HTMLButtonElement ? element : null
}

function tokenLabel(token: string) {
  if (token === '{frame}' || token.startsWith('{frame:')) return t('pathTemplate.labels.frame')

  switch (token) {
    case '{year}':
      return t('pathTemplate.labels.year')
    case '{month}':
      return t('pathTemplate.labels.month')
    case '{day}':
      return t('pathTemplate.labels.day')
    case '{hour}':
      return t('pathTemplate.labels.hour')
    case '{date}':
      return t('pathTemplate.labels.date')
    case '{user}':
      return t('pathTemplate.labels.user')
    case '{blendFileName}':
      return t('pathTemplate.labels.blendFileName')
    case '{folderName}':
      return t('pathTemplate.labels.folderName')
    case '{frameStart}':
      return t('pathTemplate.labels.frameStart')
    case '{frameEnd}':
      return t('pathTemplate.labels.frameEnd')
    default:
      return token
  }
}

function tokenMeta(token: string) {
  const parsedFrameToken = parseFrameToken(token)
  const name = token.slice(1, -1)
  const availability = (() => {
    if (parsedFrameToken) {
      if (props.kind !== 'blender') {
        return {
          known: true,
          enabled: false,
          preview: null,
          title: t('pathTemplate.unavailable.frameBlenderOnly', { token }),
        }
      }

        return {
          known: true,
          enabled: true,
          preview: '#'.repeat(parsedFrameToken.padding),
          title: `${tokenLabel(token)} → ${'#'.repeat(parsedFrameToken.padding)}`,
        }
      }

    switch (name) {
      case 'year':
      case 'month':
      case 'day':
      case 'hour':
      case 'date':
      case 'user':
      case 'frameStart':
      case 'frameEnd':
        return {
          known: true,
          enabled: true,
          preview: tokenPreview(token),
          title: tokenPreview(token) ? `${tokenLabel(token)} → ${tokenPreview(token)}` : tokenLabel(token),
        }
      case 'blendFileName':
        if (props.kind === 'standalone-ffmpeg') {
          return {
            known: true,
            enabled: false,
            preview: null,
            title: t('pathTemplate.unavailable.blendFileNameContext', { token }),
          }
        }
        if (!props.blendFileName) {
          return {
            known: true,
            enabled: true,
            preview: null,
            title: t('pathTemplate.unavailable.blendFileNamePreview', { token }),
          }
        }
        return {
          known: true,
          enabled: true,
          preview: props.blendFileName,
          title: `${tokenLabel(token)} → ${props.blendFileName}`,
        }
      case 'folderName':
        if (props.kind !== 'standalone-ffmpeg') {
          return {
            known: true,
            enabled: false,
            preview: null,
            title: t('pathTemplate.unavailable.folderNameContext', { token }),
          }
        }
        if (!props.folderName) {
          return {
            known: true,
            enabled: true,
            preview: null,
            title: t('pathTemplate.unavailable.folderNamePreview', { token }),
          }
        }
        return {
          known: true,
          enabled: true,
          preview: props.folderName,
          title: `${tokenLabel(token)} → ${props.folderName}`,
        }
      default:
        return {
          known: false,
          enabled: false,
          preview: null,
          title: t('pathTemplate.unavailable.unknown', { token }),
        }
    }
  })()

  const state: VariableState = !availability.known || !availability.enabled
    ? 'error'
    : (availability.preview ? 'normal' : 'preview')

  return {
    ...availability,
    state,
  }
}

function tokenPreview(token: string) {
  const now = new Date()
  switch (token) {
    case '{year}':
      return String(now.getFullYear())
    case '{month}':
      return String(now.getMonth() + 1).padStart(2, '0')
    case '{day}':
      return String(now.getDate()).padStart(2, '0')
    case '{hour}':
      return String(now.getHours()).padStart(2, '0')
    case '{date}':
      return `${now.getFullYear()}${String(now.getMonth() + 1).padStart(2, '0')}${String(now.getDate()).padStart(2, '0')}`
    case '{frameStart}':
      return props.frameStart != null ? String(props.frameStart) : null
    case '{frameEnd}':
      return props.frameEnd != null ? String(props.frameEnd) : null
    default:
      return null
  }
}

function parseFrameToken(token: string) {
  if (token === '{frame}') {
    return { padding: DEFAULT_FRAME_PADDING }
  }

  const matched = token.match(/^\{frame:(\d+)\}$/)
  if (!matched) return null

  const padding = Number.parseInt(matched[1] ?? '', 10)
  if (!Number.isInteger(padding) || padding < 1 || padding > MAX_FRAME_PADDING) {
    return null
  }

  return { padding }
}

function buildSegments(value: string) {
  const normalized = value ?? ''
  const segments: ParsedSegment[] = []
  let lastIndex = 0

  for (const match of normalized.matchAll(VARIABLE_PATTERN)) {
    const start = match.index ?? 0
    const token = match[0]
    if (start > lastIndex) {
      segments.push({ type: 'text', value: normalized.slice(lastIndex, start) })
    }
    const meta = tokenMeta(token)
    segments.push({
      type: 'token',
      value: token,
      state: meta.state,
      title: meta.title,
    })
    lastIndex = start + token.length
  }

  if (lastIndex < normalized.length) {
    segments.push({ type: 'text', value: normalized.slice(lastIndex) })
  }

  if (segments.length === 0) {
    segments.push({ type: 'text', value: '' })
  }

  return segments
}

function serializeEditor() {
  const editor = editorRef.value
  if (!editor) return props.modelValue ?? ''

  let result = ''
  for (const node of Array.from(editor.childNodes)) {
    if (node.nodeType === Node.TEXT_NODE) {
      result += node.textContent ?? ''
      continue
    }

    if (node.nodeType === Node.ELEMENT_NODE) {
      const element = node as HTMLElement
      if (element.dataset.token) {
        result += element.dataset.token
      } else {
        result += element.textContent ?? ''
      }
    }
  }

  return result.replace(/\u00A0/g, ' ')
}

function findSuggestionRange(value: string, offset: number) {
  const beforeCaret = value.slice(0, offset)
  const matched = beforeCaret.match(/\{([A-Za-z0-9:]*)$/)
  if (!matched) return null

  return {
    start: offset - matched[0].length,
    end: offset,
    query: matched[1] ?? '',
  }
}

function serializedLength(node: Node | null | undefined) {
  if (!node) return 0
  if (node.nodeType === Node.TEXT_NODE) {
    return node.textContent?.replace(/\u00A0/g, ' ').length ?? 0
  }

  if (node.nodeType === Node.ELEMENT_NODE) {
    const element = node as HTMLElement
    if (element.dataset.token) {
      return element.dataset.token.length
    }
  }

  return node.textContent?.replace(/\u00A0/g, ' ').length ?? 0
}

function closestEditorChild(node: Node | null, editor: HTMLElement) {
  let current = node
  while (current && current.parentNode !== editor) {
    current = current.parentNode
  }
  return current
}

function offsetBeforeChild(editor: HTMLElement, child: Node | null) {
  let total = 0
  for (const node of Array.from(editor.childNodes)) {
    if (node === child) {
      break
    }
    total += serializedLength(node)
  }
  return total
}

function boundaryOffset(editor: HTMLElement, container: Node, offset: number) {
  if (container === editor) {
    let total = 0
    for (let index = 0; index < offset; index += 1) {
      total += serializedLength(editor.childNodes.item(index))
    }
    return total
  }

  const editorChild = closestEditorChild(container, editor)
  if (!editorChild) {
    return serializeEditor().length
  }

  const baseOffset = offsetBeforeChild(editor, editorChild)
  if (editorChild.nodeType === Node.TEXT_NODE) {
    return baseOffset + offset
  }

  const tokenLength = serializedLength(editorChild)
  if (editorChild.contains(container)) {
    return baseOffset + (offset > 0 ? tokenLength : 0)
  }

  return baseOffset
}

function renderEditor(value: string, caretOffset?: number | null) {
  const editor = editorRef.value
  if (!editor) return

  const fragment = document.createDocumentFragment()
  for (const segment of buildSegments(value)) {
    if (segment.type === 'text') {
      fragment.append(document.createTextNode(segment.value))
      continue
    }

    const pill = document.createElement('span')
    pill.className = `path-template-pill path-template-pill-${segment.state}`
    pill.dataset.token = segment.value
    pill.contentEditable = 'false'
    pill.title = segment.title
    pill.textContent = segment.value.slice(1, -1)
    fragment.append(pill)
  }

  editor.replaceChildren(fragment)
  hasContent.value = value.length > 0

  if (!props.disabled && typeof caretOffset === 'number') {
    restoreCaretOffset(caretOffset)
  }
}

function syncLastCaretOffset() {
  lastCaretOffset.value = getCaretOffset()
}

function refreshSuggestions() {
  if (props.disabled) {
    showSuggestions.value = false
    suggestionQuery.value = ''
    suggestionRange.value = null
    selectedSuggestionIndex.value = 0
    suggestionItemRefs.value = []
    return
  }

  const value = serializeEditor()
  const offset = lastCaretOffset.value ?? value.length
  const context = findSuggestionRange(value, offset)
  if (!context) {
    showSuggestions.value = false
    suggestionQuery.value = ''
    suggestionRange.value = null
    selectedSuggestionIndex.value = 0
    suggestionItemRefs.value = []
    return
  }

  const previousQuery = suggestionQuery.value
  const previousRange = suggestionRange.value

  suggestionQuery.value = context.query
  suggestionRange.value = { start: context.start, end: context.end }
  showSuggestions.value = filteredSuggestions.value.length > 0

  if (!filteredSuggestions.value.length) {
    selectedSuggestionIndex.value = 0
    return
  }

  const sameContext = previousQuery === context.query
    && previousRange?.start === context.start
    && previousRange?.end === context.end

  selectedSuggestionIndex.value = sameContext
    ? Math.min(selectedSuggestionIndex.value, filteredSuggestions.value.length - 1)
    : 0
}

function focusEditor() {
  editorRef.value?.focus()
}

function syncSuggestionScroll() {
  if (!showSuggestions.value || !filteredSuggestions.value.length) return
  nextTick(() => {
    suggestionItemRefs.value[selectedSuggestionIndex.value]?.scrollIntoView({
      block: 'nearest',
      inline: 'nearest',
    })
  })
}

function getCaretOffset() {
  const editor = editorRef.value
  const selection = window.getSelection()
  if (!editor || !selection || selection.rangeCount === 0) return lastCaretOffset.value ?? 0
  const range = selection.getRangeAt(0)
  if (!editor.contains(range.startContainer)) {
    return lastCaretOffset.value ?? serializeEditor().length
  }

  return boundaryOffset(editor, range.startContainer, range.startOffset)
}

function getSelectionOffsets() {
  const editor = editorRef.value
  const selection = window.getSelection()
  if (!editor || !selection || selection.rangeCount === 0) return null

  const range = selection.getRangeAt(0)
  const startInside = range.startContainer === editor || editor.contains(range.startContainer)
  const endInside = range.endContainer === editor || editor.contains(range.endContainer)
  if (!startInside || !endInside) return null

  const start = boundaryOffset(editor, range.startContainer, range.startOffset)
  const end = boundaryOffset(editor, range.endContainer, range.endOffset)

  return start <= end ? { start, end } : { start: end, end: start }
}

function restoreCaretOffset(offset: number) {
  const editor = editorRef.value
  if (!editor) return

  const selection = window.getSelection()
  if (!selection) return

  let remaining = Math.max(0, offset)
  const range = document.createRange()

  const children = Array.from(editor.childNodes)
  for (const node of children) {
    const nodeLength = serializedLength(node)
    if (remaining > nodeLength) {
      remaining -= nodeLength
      continue
    }

    if (node.nodeType === Node.TEXT_NODE) {
      range.setStart(node, remaining)
      range.collapse(true)
    } else {
      const parent = node.parentNode ?? editor
      const index = Array.prototype.indexOf.call(parent.childNodes, node)
      range.setStart(parent, remaining === 0 ? index : index + 1)
      range.collapse(true)
    }

    selection.removeAllRanges()
    selection.addRange(range)
    lastCaretOffset.value = offset
    return
  }

  range.selectNodeContents(editor)
  range.collapse(false)
  selection.removeAllRanges()
  selection.addRange(range)
  lastCaretOffset.value = serializeEditor().length
}

function applyValue(value: string, caretOffset?: number | null) {
  emit('update:modelValue', value)
  nextTick(() => {
    renderEditor(value, caretOffset)
    refreshSuggestions()
  })
}

function insertToken(token: string, offset = lastCaretOffset.value ?? serializeEditor().length) {
  const current = serializeEditor()
  const nextValue = `${current.slice(0, offset)}${token}${current.slice(offset)}`
  applyValue(nextValue, offset + token.length)
}

function replaceRange(start: number, end: number, replacement: string) {
  const current = serializeEditor()
  const nextValue = `${current.slice(0, start)}${replacement}${current.slice(end)}`
  applyValue(nextValue, start + replacement.length)
}

function handleInput() {
  const nextValue = serializeEditor()
  const caretOffset = getCaretOffset()
  applyValue(nextValue, caretOffset)
}

function handleFocus() {
  syncLastCaretOffset()
  refreshSuggestions()
  emit('focus')
}

function handlePointerSync() {
  syncLastCaretOffset()
  refreshSuggestions()
  emit('focus')
}

function handleCaretSync(event: KeyboardEvent) {
  if (['ArrowDown', 'ArrowUp', 'Enter', 'Tab', 'Escape'].includes(event.key)) {
    return
  }

  syncLastCaretOffset()
  refreshSuggestions()
}

function handlePaste(event: ClipboardEvent) {
  event.preventDefault()
  if (props.disabled) return
  const text = event.clipboardData?.getData('text/plain') ?? ''
  const selection = getSelectionOffsets()
  if (selection) {
    replaceRange(selection.start, selection.end, text)
    return
  }
  const offset = getCaretOffset()
  replaceRange(offset, offset, text)
}

function handleCopy(event: ClipboardEvent) {
  const selection = getSelectionOffsets()
  if (!selection || selection.start === selection.end) return

  event.preventDefault()
  event.clipboardData?.setData('text/plain', serializeEditor().slice(selection.start, selection.end))
}

function handleKeydown(event: KeyboardEvent) {
  if (props.disabled) return
  if (showSuggestions.value && filteredSuggestions.value.length) {
    if (event.key === 'ArrowDown') {
      event.preventDefault()
      selectedSuggestionIndex.value = (selectedSuggestionIndex.value + 1) % filteredSuggestions.value.length
      return
    }

    if (event.key === 'ArrowUp') {
      event.preventDefault()
      selectedSuggestionIndex.value = (selectedSuggestionIndex.value + filteredSuggestions.value.length - 1) % filteredSuggestions.value.length
      return
    }

    if (event.key === 'Enter' || event.key === 'Tab') {
      event.preventDefault()
      applySuggestion(filteredSuggestions.value[selectedSuggestionIndex.value]?.token)
      return
    }

    if (event.key === 'Escape') {
      event.preventDefault()
      showSuggestions.value = false
      suggestionRange.value = null
      return
    }
  }

  if (event.key === 'Enter') {
    event.preventDefault()
    return
  }

  const current = serializeEditor()
  const offset = getCaretOffset()
  if (event.key === 'Backspace' && offset > 0) {
    const matched = current.slice(0, offset).match(/\{(?:[A-Za-z][A-Za-z0-9]*|frame:\d+)\}$/)
    if (matched) {
      event.preventDefault()
      replaceRange(offset - matched[0].length, offset, '')
    }
  }

  if (event.key === 'Delete' && offset < current.length) {
    const matched = current.slice(offset).match(/^\{(?:[A-Za-z][A-Za-z0-9]*|frame:\d+)\}/)
    if (matched) {
      event.preventDefault()
      replaceRange(offset, offset + matched[0].length, '')
    }
  }
}

function handleVariableDragStart(event: DragEvent, variable: VariableItem) {
  if (!variable.enabled) {
    event.preventDefault()
    return
  }

  event.dataTransfer?.setData(TOKEN_DRAG_MIME, variable.token)
  event.dataTransfer?.setData('text/plain', variable.token)
  if (event.dataTransfer) {
    event.dataTransfer.effectAllowed = 'copy'
  }
}

function handleDragOver(event: DragEvent) {
  if (props.disabled) return
  event.preventDefault()
  dragOver.value = true
  if (event.dataTransfer) {
    event.dataTransfer.dropEffect = 'copy'
  }
}

function caretOffsetFromPoint(clientX: number, clientY: number) {
  const selection = window.getSelection()
  const editor = editorRef.value
  if (!editor || !selection) return serializeEditor().length

  const caretPosition = document.caretPositionFromPoint?.(clientX, clientY)
  if (caretPosition && editor.contains(caretPosition.offsetNode)) {
    selection.removeAllRanges()
    const range = document.createRange()
    range.setStart(caretPosition.offsetNode, caretPosition.offset)
    range.collapse(true)
    selection.addRange(range)
    return getCaretOffset()
  }

  const legacyRange = document.caretRangeFromPoint?.(clientX, clientY)
  if (legacyRange && editor.contains(legacyRange.startContainer)) {
    selection.removeAllRanges()
    selection.addRange(legacyRange)
    return getCaretOffset()
  }

  return serializeEditor().length
}

function handleDrop(event: DragEvent) {
  if (props.disabled) return
  event.preventDefault()
  dragOver.value = false
  const token = event.dataTransfer?.getData(TOKEN_DRAG_MIME) || event.dataTransfer?.getData('text/plain') || ''
  if (!TOKEN_PATTERN.test(token)) return

  const offset = caretOffsetFromPoint(event.clientX, event.clientY)
  insertToken(token, offset)
}

function applySuggestion(token?: string) {
  if (!token) return

  const range = suggestionRange.value
  if (!range) {
    insertToken(token)
    return
  }

  replaceRange(range.start, range.end, token)
}

const variableGroups = computed(() => {
  const groups: Array<{ label: string, items: VariableItem[] }> = [
    {
      label: t('pathTemplate.groups.project'),
      items: ['{blendFileName}', '{folderName}'].map(token => {
        const meta = tokenMeta(token)
        return {
          token,
          label: token,
          enabled: meta.enabled,
          preview: meta.preview,
          description: meta.title,
        }
      }),
    },
    {
      label: t('pathTemplate.groups.frame'),
      items: ['{frame}', '{frameStart}', '{frameEnd}'].map(token => {
        const meta = tokenMeta(token)
        return {
          token,
          label: token,
          enabled: meta.enabled,
          preview: meta.preview,
          description: meta.title,
        }
      }),
    },
    {
      label: t('pathTemplate.groups.time'),
      items: ['{date}', '{year}', '{month}', '{day}', '{hour}'].map(token => ({
        token,
        label: token,
        enabled: true,
        preview: tokenPreview(token),
        description: tokenPreview(token) ? `${tokenLabel(token)} → ${tokenPreview(token)}` : tokenLabel(token),
      })),
    },
    {
      label: t('pathTemplate.groups.system'),
      items: [
        {
          token: '{user}',
          label: '{user}',
          enabled: true,
          preview: null,
          description: tokenLabel('{user}'),
        },
      ],
    },
  ]

  return groups
})

const availableVariables = computed(() =>
  variableGroups.value
    .flatMap(group => group.items)
    .filter(variable => variable.enabled),
)

const filteredSuggestions = computed(() => {
  const query = suggestionQuery.value.trim().toLowerCase()
  if (!query) return availableVariables.value

  return availableVariables.value.filter((variable) => {
      const normalizedToken = variable.token.replace(/[{}]/g, '').toLowerCase()
      return normalizedToken.includes(query) || variable.description.toLowerCase().includes(query)
    })
})

watch(selectedSuggestionIndex, () => {
  syncSuggestionScroll()
})

watch(
  () => filteredSuggestions.value.length,
  () => {
    suggestionItemRefs.value = suggestionItemRefs.value.slice(0, filteredSuggestions.value.length)
    syncSuggestionScroll()
  },
)

watch(
  () => props.modelValue,
  value => {
    const current = serializeEditor()
    if (value === current) {
      hasContent.value = value.length > 0
      return
    }
    renderEditor(value ?? '', null)
    nextTick(refreshSuggestions)
  },
)

watch(
  () => [props.kind, props.blendFileName, props.folderName, props.frameStart, props.frameEnd] as const,
  () => {
    renderEditor(props.modelValue ?? '', lastCaretOffset.value)
    nextTick(refreshSuggestions)
  },
)

watch(locale, () => {
  renderEditor(props.modelValue ?? '', lastCaretOffset.value)
  nextTick(refreshSuggestions)
})

onMounted(() => {
  renderEditor(props.modelValue ?? '', null)
  refreshSuggestions()
})

watch(
  () => props.variablePanelMode,
  value => {
    if (value === 'expanded') {
      showVariables.value = true
    } else if (value === 'hidden') {
      showVariables.value = false
    }
  },
)

defineExpose({
  insertToken,
  focusEditor,
})
</script>

<style scoped>
.path-template-input {
  display: grid;
  gap: 0.45rem;
}

.path-template-editor {
  min-height: 2.7rem;
  padding: 0.62rem 0.78rem;
  color: var(--ui-text);
  font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
  font-size: 0.9rem;
  line-height: 1.45;
  white-space: pre-wrap;
  word-break: break-word;
  cursor: text;
}

.path-template-editor:focus {
  outline: none;
  border-color: color-mix(in srgb, var(--ui-primary) 38%, var(--ui-border-accented));
  box-shadow: 0 0 0 1px color-mix(in srgb, var(--ui-primary) 16%, transparent);
}

.path-template-editor-empty::before {
  content: attr(data-placeholder);
  color: var(--ui-text-dimmed);
  pointer-events: none;
}

.path-template-editor-disabled {
  opacity: 0.7;
  cursor: not-allowed;
}

.path-template-editor-dragover {
  border-color: color-mix(in srgb, var(--ui-primary) 38%, var(--ui-border-accented));
  background: color-mix(in srgb, var(--ui-primary) 6%, var(--ui-bg-muted));
}

:deep(.path-template-pill) {
  display: inline-flex;
  align-items: center;
  margin: 0 0.12rem;
  padding: 0.08rem 0.45rem;
  border-radius: 999px;
  border: 1px solid transparent;
  font-size: 0.82rem;
  font-weight: 600;
  line-height: 1.4;
  user-select: none;
}

:deep(.path-template-pill-normal) {
  border-color: color-mix(in srgb, var(--ui-primary) 28%, transparent);
  background: color-mix(in srgb, var(--ui-primary) 12%, var(--ui-bg-muted));
  color: var(--ui-primary);
}

:deep(.path-template-pill-preview) {
  border-color: color-mix(in srgb, var(--ui-border-accented) 60%, transparent);
  background: color-mix(in srgb, var(--ui-bg-elevated) 92%, transparent);
  color: var(--ui-text-muted);
}

:deep(.path-template-pill-error) {
  border-color: color-mix(in srgb, var(--ui-error) 28%, transparent);
  background: color-mix(in srgb, var(--ui-error) 10%, var(--ui-bg-muted));
  color: var(--ui-error);
}

.path-template-toolbar {
  display: flex;
  justify-content: flex-end;
}

.path-template-suggestion-menu {
  display: grid;
  gap: 0.22rem;
  max-height: 16rem;
  padding: 0.35rem;
  overflow-y: auto;
}

.path-template-suggestion-item {
  display: grid;
  gap: 0.12rem;
  width: 100%;
  padding: 0.55rem 0.7rem;
  border: 0;
  border-radius: 0.75rem;
  background: transparent;
  text-align: left;
  cursor: pointer;
}

.path-template-suggestion-item:hover,
.path-template-suggestion-item-active {
  background: color-mix(in srgb, var(--ui-primary) 10%, var(--ui-bg-elevated));
}

.path-template-suggestion-token {
  color: var(--ui-text-highlighted);
  font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
  font-size: 0.84rem;
  font-weight: 700;
}

.path-template-suggestion-description {
  color: var(--ui-text-muted);
  font-size: 0.74rem;
  line-height: 1.4;
}

.path-template-panel {
  display: grid;
  gap: 0.7rem;
  padding: 0.75rem;
}

.path-template-group {
  display: grid;
  gap: 0.45rem;
}

.path-template-group-label {
  margin: 0;
  color: var(--ui-text-dimmed);
  font-size: 0.72rem;
  font-weight: 700;
  letter-spacing: 0.06em;
  text-transform: uppercase;
}

.path-template-chip-row {
  display: flex;
  flex-wrap: wrap;
  gap: 0.45rem;
}
</style>
