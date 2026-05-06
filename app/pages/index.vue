<template>
  <div class="queue-page">
    <Transition name="drop-fade">
      <div v-if="isDragging && !draggedJobId" class="drop-overlay">
        <div class="drop-message">
          <UIcon name="i-lucide-download" class="drop-icon" />
          <div class="drop-copy">
            <strong>拖拽 .blend 工程到窗口</strong>
            <span>松开以创建渲染任务</span>
          </div>
        </div>
      </div>
    </Transition>

    <section class="queue-header">
      <section class="page-hero queue-hero">
        <div class="page-hero-copy">
          <div class="queue-title-row">
            <UBadge label="Render Queue" color="neutral" variant="subtle" class="page-eyebrow" />
            <UBadge
              :label="jobsStore.queuePaused ? '队列已暂停' : '队列运行中'"
              :color="jobsStore.queuePaused ? 'warning' : 'success'"
              variant="subtle"
            />
          </div>
          <div class="queue-heading-row">
            <div class="queue-heading-copy">
              <h1>渲染队列</h1>
              <p class="page-note">管理本地 Blender 渲染任务。</p>
            </div>
            <div class="queue-hero-actions-stack">
              <div class="page-hero-actions queue-hero-actions queue-hero-actions-secondary">
                <UTooltip :text="queueToggleTooltip" arrow :content="{ side: 'bottom', sideOffset: 8 }">
                  <UButton
                    :icon="queueToggleButton.icon"
                    :label="queueToggleButton.label"
                    :color="queueToggleButton.color"
                    variant="outline"
                    :disabled="queueToggleDisabled"
                    @click="handleQueueToggle"
                  />
                </UTooltip>
              </div>
              <div class="page-hero-actions queue-hero-actions">
                <UFieldGroup size="md">
                  <UTooltip text="创建新的渲染任务" arrow :content="{ side: 'bottom', sideOffset: 8 }">
                    <UButton icon="i-lucide-plus" label="新建任务" color="primary" variant="solid" @click="openAddJob" />
                  </UTooltip>
                  <UDropdownMenu
                    :items="renderQueueActionItems"
                    arrow
                    :content="{ side: 'bottom', align: 'end', sideOffset: 8 }"
                  >
                    <UButton
                      icon="i-lucide-chevron-down"
                      color="neutral"
                      variant="outline"
                      square
                      :disabled="clearingCompletedRenderJobs"
                    />
                  </UDropdownMenu>
                </UFieldGroup>
              </div>
            </div>
          </div>
        </div>
      </section>

      <div class="queue-tabs-row surface-panel">
        <UTabs
          v-model="activeQueueTab"
          :items="queueTabItems"
          :content="false"
          color="neutral"
          variant="pill"
          class="queue-tabs"
          :ui="{
            indicator: 'hidden',
            list: 'queue-tabs-list',
            trigger: 'queue-tab-trigger',
            label: 'queue-tab-label',
          }"
        />
      </div>

      <UAlert v-if="retryActionError" color="error" variant="subtle" :description="retryActionError" class="surface-alert" />
    </section>

    <section class="queue-content">
      <div v-if="jobsStore.loading" class="loading">加载中…</div>

        <UCard v-else-if="jobsStore.jobs.length === 0" variant="subtle" class="empty-state" :ui="{ body: 'empty-state-body' }">
        <div class="empty-state-icon">
          <UIcon name="i-lucide-film" />
        </div>
        <div class="empty-state-title">还没有渲染任务</div>
        <div class="empty-state-note">拖拽 .blend 工程到窗口，或点击“新建任务”按钮创建渲染任务。</div>
      </UCard>

      <UCard
        v-else-if="filteredJobs.length === 0"
        variant="subtle"
        class="empty-state queue-empty-state"
        :class="emptyTabToneClass"
        :ui="{ body: 'empty-state-body' }"
      >
        <div class="empty-state-icon">
          <UIcon name="i-lucide-filter" />
        </div>
        <div class="empty-state-title">{{ emptyTabTitle }}</div>
        <div class="empty-state-note">{{ emptyTabNote }}</div>
      </UCard>

      <TransitionGroup v-else name="job-list" tag="div" class="job-list">
      <div
        v-for="job in filteredJobs"
        :key="job.id"
        :data-job-id="job.id"
        class="job-list-item"
        :class="{
          'job-list-item-draggable': canDragQueueJob(job),
          'job-list-item-dragging': draggedJobId === job.id,
          'job-list-item-drop-before': dropTargetJobId === job.id && dropPosition === 'before',
          'job-list-item-drop-after': dropTargetJobId === job.id && dropPosition === 'after',
        }"
        @pointerdown="handleQueuePointerDown(job, $event)"
      >
        <UContextMenu
          :items="buildJobContextMenuItems(job)"
          :ui="{ content: 'job-context-menu-content' }"
        >
          <div class="job-context-menu-target" data-context-menu>
            <JobCard
              :job="job"
              @cancel="jobsStore.stopJob(job.id)"
              @remove="deleteConfirmJob = job"
              @retry="handleRetry(job)"
            />
          </div>
        </UContextMenu>
      </div>
      </TransitionGroup>
    </section>

    <JobMetadataModal
      v-model:open="metadataDialogOpen"
      :job="metadataJob"
    />

    <UModal
      :open="showRetryConfirm"
      :close="false"
      title="选择渲染方式"
      :ui="{ content: 'job-modal-content retry-modal-content' }"
      @update:open="v => { if (!v) closeRetryConfirm() }"
    >
      <template #body>
        <div class="modal-stack">
          <div class="transcode-submit-stack retry-modal-stack">
            <div v-if="retryIsQuickMp4" class="choice-grid retry-choice-grid">
              <section class="surface-panel transcode-submit-section retry-option-section retry-option-section-wide">
                <div class="transcode-submit-head">
                  <div>
                    <p class="choice-card-mode">快速 MP4</p>
                    <h3 class="choice-card-title">重新渲染整个片段</h3>
                  </div>
                </div>
                <p class="choice-card-desc">
                  快速 MP4 为单文件输出，仅支持整段覆盖重新渲染。
                </p>
                <div class="choice-card-toggle-group">
                  <UButton
                    icon="i-lucide-refresh-ccw"
                    label="整段覆盖"
                    color="neutral"
                    variant="outline"
                    size="sm"
                    class="choice-card-toggle-button"
                    :loading="retrySubmittingMode === 'restart'"
                    :disabled="retrySubmittingMode !== null"
                    @click="confirmRetryFromStart"
                  />
                </div>
                <div class="choice-card-note-stack">
                  <p class="choice-card-inline-note">{{ retryFullRangeSummary }}</p>
                </div>
              </section>
            </div>
            <div v-else class="choice-grid retry-choice-grid">
              <section class="surface-panel transcode-submit-section retry-option-section retry-option-section-wide">
                <div class="transcode-submit-head">
                  <div>
                    <p class="choice-card-mode">整体片段</p>
                    <h3 class="choice-card-title">重跑整个片段</h3>
                  </div>
                </div>
                <p class="choice-card-desc">
                  处理完整片段 <span class="choice-card-accent">{{ retryFullRangeLabel }}</span>。
                </p>
                <div class="choice-card-toggle-group">
                  <UButton
                    icon="i-lucide-chevrons-right"
                    label="整段续跑"
                    color="neutral"
                    variant="outline"
                    size="sm"
                    class="choice-card-toggle-button"
                    :loading="retrySubmittingMode === 'continue'"
                    :disabled="retrySubmittingMode !== null || retryConfirmJob?.status === 'done'"
                    @mouseenter="retryFullRangePreviewMode = 'continue'"
                    @mouseleave="clearRetryPreviewOnLeave($event, 'full')"
                    @focus="retryFullRangePreviewMode = 'continue'"
                    @blur="clearRetryPreviewOnLeave($event, 'full')"
                    @click="confirmRetryContinue"
                  />
                  <UButton
                    icon="i-lucide-refresh-ccw"
                    label="整段覆盖"
                    color="neutral"
                    variant="outline"
                    size="sm"
                    class="choice-card-toggle-button"
                    :loading="retrySubmittingMode === 'restart'"
                    :disabled="retrySubmittingMode !== null"
                    @mouseenter="retryFullRangePreviewMode = 'restart'"
                    @mouseleave="clearRetryPreviewOnLeave($event, 'full')"
                    @focus="retryFullRangePreviewMode = 'restart'"
                    @blur="clearRetryPreviewOnLeave($event, 'full')"
                    @click="confirmRetryFromStart"
                  />
                </div>
                <div class="choice-card-note-stack">
                  <p class="choice-card-inline-note">{{ retryFullRangeSummary }}</p>
                </div>
              </section>

              <section class="surface-panel transcode-submit-section retry-option-section retry-option-section-wide">
                <div class="transcode-submit-head">
                  <div>
                    <p class="choice-card-mode">指定区间</p>
                    <h3 class="choice-card-title">只重跑需要修复的部分</h3>
                  </div>
                </div>
                <p class="choice-card-desc">
                  只处理你填写的帧范围，适合补帧或局部返修。
                </p>
                <div class="choice-card-fields">
                  <UFormField label="起始帧">
                    <UInputNumber v-model="retryCustomStart" :min="1" />
                  </UFormField>
                  <UFormField label="结束帧">
                    <UInputNumber v-model="retryCustomEnd" :min="1" />
                  </UFormField>
                </div>
                <div class="choice-card-toggle-group">
                  <UButton
                    icon="i-lucide-chevrons-right"
                    label="按区间续跑"
                    color="neutral"
                    variant="outline"
                    size="sm"
                    class="choice-card-toggle-button"
                    :loading="retrySubmittingMode === 'range-continue'"
                    :disabled="retrySubmittingMode !== null"
                    @mouseenter="retryCustomPreviewMode = 'continue'"
                    @mouseleave="clearRetryPreviewOnLeave($event, 'custom')"
                    @focus="retryCustomPreviewMode = 'continue'"
                    @blur="clearRetryPreviewOnLeave($event, 'custom')"
                    @click="confirmRetryCustomRange(true)"
                  />
                  <UButton
                    icon="i-lucide-refresh-ccw"
                    label="按区间覆盖"
                    color="neutral"
                    variant="outline"
                    size="sm"
                    class="choice-card-toggle-button"
                    :loading="retrySubmittingMode === 'range-restart'"
                    :disabled="retrySubmittingMode !== null"
                    @mouseenter="retryCustomPreviewMode = 'restart'"
                    @mouseleave="clearRetryPreviewOnLeave($event, 'custom')"
                    @focus="retryCustomPreviewMode = 'restart'"
                    @blur="clearRetryPreviewOnLeave($event, 'custom')"
                    @click="confirmRetryCustomRange(false)"
                  />
                </div>
                <div class="choice-card-note-stack">
                  <p class="choice-card-inline-note">{{ retryCustomActionDescription }}</p>
                  <p v-if="retryCustomInspectLoading" class="choice-card-inline-note">正在检查所选区间…</p>
                  <p v-else-if="retryCustomRangeSummary" class="choice-card-inline-note">{{ retryCustomRangeSummary }}</p>
                </div>
                <UAlert
                  v-if="retryActionError"
                  color="error"
                  variant="subtle"
                  :description="retryActionError"
                />
              </section>
            </div>

            <section v-if="retryConfirmJob && !retryIsQuickMp4 && !['OPEN_EXR', 'EXR'].includes(retryConfirmJob.outputFormat)" class="surface-panel transcode-submit-section retry-transcode-panel">
              <div class="retry-transcode-head">
                <p class="choice-card-mode">自动转码</p>
                <h3 class="choice-card-title">补渲后自动创建转码任务</h3>
              </div>
              <p class="choice-card-desc">
                决定这次补渲结束后，是否自动生成 FFmpeg Job。
              </p>
              <USwitch
                v-model="retryAutoTranscodeEnabled"
                color="neutral"
                label="补渲完成后自动转码"
                class="choice-card-switch"
              />
              <div class="choice-card-toggle-group">
                <UButton
                  icon="i-lucide-scan-line"
                  :label="`目标片段 ${retryCurrentTargetRangeLabel}`"
                  :color="retryTranscodeRangeMode === 'current' ? 'primary' : 'neutral'"
                  :variant="retryTranscodeRangeMode === 'current' ? 'solid' : 'outline'"
                  size="sm"
                  class="choice-card-toggle-button"
                  :disabled="retrySubmittingMode !== null || !retryAutoTranscodeEnabled"
                  @click="retryTranscodeRangeMode = 'current'"
                />
                <UButton
                  icon="i-lucide-film"
                  :label="`${retrySavedTranscodeRangeTitle} ${retryOriginalTranscodeRangeLabel}`"
                  :color="retryTranscodeRangeMode === 'original' ? 'primary' : 'neutral'"
                  :variant="retryTranscodeRangeMode === 'original' ? 'solid' : 'outline'"
                  size="sm"
                  class="choice-card-toggle-button"
                  :disabled="retrySubmittingMode !== null || !retryAutoTranscodeEnabled"
                  @click="retryTranscodeRangeMode = 'original'"
                />
              </div>
              <div class="choice-card-note-stack">
                <p class="choice-card-inline-note">{{ retryTranscodeSummary }}</p>
              </div>
            </section>
          </div>
          <div class="modal-actions">
            <UButton icon="i-lucide-x" label="取消" color="warning" variant="outline" size="sm" :disabled="retrySubmittingMode !== null" @click="closeRetryConfirm" />
          </div>
        </div>
      </template>
    </UModal>

    <UModal
      :open="!!deleteConfirmJob"
      :close="false"
      title="删除任务"
      :ui="{ content: 'job-modal-content' }"
      @update:open="v => { if (!v) deleteConfirmJob = null }"
    >
      <template #body>
        <div class="modal-stack">
          <p class="modal-copy">
            确定删除 <strong>{{ deleteConfirmJob?.name }}</strong>？此操作不可撤销。
          </p>
          <div class="modal-actions">
            <UButton icon="i-lucide-x" label="取消" color="warning" variant="outline" @click="deleteConfirmJob = null" />
            <UButton icon="i-lucide-trash-2" label="删除" color="error" variant="outline" @click="confirmDelete" />
          </div>
        </div>
      </template>
    </UModal>

    <UModal
      :open="showClearCompletedConfirm"
      :close="false"
      title="清理已完成任务"
      :ui="{ content: 'job-modal-content' }"
      @update:open="v => { if (!v) showClearCompletedConfirm = false }"
    >
      <template #body>
        <div class="modal-stack">
          <p class="modal-copy">
            确定清理当前所有
            <strong>{{ jobsStore.doneJobs.length }}</strong>
            个已完成渲染任务？此操作不可撤销。
          </p>
          <div class="modal-actions">
            <UButton
              icon="i-lucide-x"
              label="取消"
              color="warning"
              variant="outline"
              :disabled="clearingCompletedRenderJobs"
              @click="showClearCompletedConfirm = false"
            />
            <UButton
              icon="i-lucide-trash-2"
              label="确认清理"
              color="error"
              variant="outline"
              :loading="clearingCompletedRenderJobs"
              @click="confirmClearCompletedRenderJobs"
            />
          </div>
        </div>
      </template>
    </UModal>

    <UModal
      :open="showAddJob"
      title="新建渲染任务"
      :ui="{ content: 'job-modal-content job-form-modal' }"
      @update:open="v => { if (!v) closeAddJob() }"
    >
      <template #body>
        <form class="modal-stack" @submit.prevent="submitNewJob">
          <div class="job-form-modern-stack">
            <section class="surface-panel transcode-submit-section">
              <div class="transcode-submit-head">
                <div>
                  <p class="choice-card-mode">基础项</p>
                  <h2 class="choice-card-title">项目与输出</h2>
                </div>
              </div>

              <div class="job-form-fields">
                <UFormField label="任务名称" size="lg" class="job-form-field">
                  <UTextarea v-model.trim="form.name" :rows="1" autoresize class="job-name-textarea" placeholder="潜行瞬鲨_镜头010_终版" :ui="{ root: 'w-full' }" />
                </UFormField>

                <UFormField label="任务备注" size="lg" class="job-form-field">
                  <UTextarea
                    v-model="formNote"
                    :rows="2"
                    autoresize
                    class="path-textarea"
                    placeholder="例如：重渲 120-180 帧，使用 OpenEXR 输出，完成后自动转码。"
                    :ui="{ root: 'w-full' }"
                  />
                </UFormField>

                <UFormField label="Blend 文件" class="job-form-field">
                  <div class="transcode-submit-path-row">
                    <UTextarea v-model.trim="form.blend_file" :rows="1" autoresize class="w-full path-textarea path-textarea-lg" placeholder="F:\项目\潜行瞬鲨\潜行瞬鲨.blend" :ui="{ root: 'w-full' }" />
                    <UButton type="button" icon="i-lucide-folder-open" label="浏览" color="neutral" variant="outline" @click="browseBlendFile" />
                  </div>
                </UFormField>

                <UFormField label="输出路径" class="job-form-field">
                  <div class="transcode-submit-path-row">
                    <UTextarea
                      v-model.trim="form.output_path"
                      :rows="1"
                      autoresize
                      class="w-full path-textarea path-textarea-xl"
                      :placeholder="outputPathPlaceholder"
                      :ui="{ root: 'w-full' }"
                    />
                    <UButton type="button" icon="i-lucide-folder-open" label="浏览" color="neutral" variant="outline" @click="browseRenderOutputDirectory" />
                  </div>
                </UFormField>
              </div>
            </section>

            <section class="surface-panel transcode-submit-section">
              <div class="transcode-submit-head">
                <div>
                  <p class="choice-card-mode">渲染控制</p>
                  <h2 class="choice-card-title">执行参数</h2>
                </div>
              </div>

              <div v-if="settingsStore.blenderVersions.length" class="job-form-fields">
                <UFormField label="Blender 版本" class="job-form-field">
                  <div class="chip-row version-chip-row">
                    <UButton
                      v-for="b in settingsStore.blenderVersions"
                      :key="b.executable"
                      type="button"
                      size="sm"
                      :color="form.blender_executable === b.executable ? 'primary' : 'neutral'"
                      :variant="form.blender_executable === b.executable ? 'solid' : 'outline'"
                      :label="`Blender ${b.version}`"
                      @click="selectBlender(b.executable)"
                    />
                  </div>
                </UFormField>

                <div class="job-form-modern-grid">
                  <UFormField label="格式" class="job-format-field">
                    <USelect
                      v-model="selectedOutputMode"
                      :items="outputFormatOptions"
                      trailing-icon="i-lucide-chevron-down"
                      class="job-format-select"
                      :ui="{
                        base: 'w-full pe-9',
                        trailing: 'pointer-events-none absolute inset-y-0 end-0 flex items-center pe-3',
                        trailingIcon: 'size-4 text-primary',
                        item: 'relative justify-center',
                        itemLabel: 'w-full text-center',
                        itemTrailing: 'absolute end-2'
                      }"
                    >
                      <template #default="{ modelValue }">
                        <span class="min-w-0 flex-1 truncate opacity-0 pointer-events-none select-none" aria-hidden="true">
                          {{ outputModeLabelMap[modelValue as OutputMode] || '选择格式' }}
                        </span>
                        <span class="absolute inset-0 flex items-center justify-center px-6 pointer-events-none">
                          {{ outputModeLabelMap[modelValue as OutputMode] || '选择格式' }}
                        </span>
                      </template>
                    </USelect>
                  </UFormField>

                  <UFormField label="起始帧">
                    <UInputNumber v-model="form.frame_start" :min="1" :ui="{ root: 'w-full' }" />
                  </UFormField>

                  <UFormField label="结束帧">
                    <UInputNumber v-model="form.frame_end" :min="1" :ui="{ root: 'w-full' }" />
                  </UFormField>
                </div>

                <UFormField label="输出参数">
                  <div class="job-form-transcode-panel surface-panel">
                    <div class="job-form-transcode-toggle-row">
                      <div class="job-form-transcode-copy">
                        <p class="job-form-transcode-title">{{ outputSettingsTitle }}</p>
                        <p class="hint-text">{{ outputSettingsSummary }}</p>
                      </div>
                      <UButton
                        v-if="!isQuickMp4Output"
                        type="button"
                        icon="i-lucide-image-up"
                        label="输出设置"
                        color="neutral"
                        variant="outline"
                        @click="addJobOutputSettingsOpen = true"
                      />
                    </div>
                  </div>
                </UFormField>

                <UFormField v-if="!isQuickMp4Output" label="渲染后转码">
                  <div class="job-form-transcode-panel surface-panel">
                    <div class="job-form-transcode-toggle-row">
                      <div class="job-form-transcode-copy">
                        <p class="job-form-transcode-title">自动提交到转码队列</p>
                        <p class="hint-text">
                          {{ isExrOutput ? 'OpenEXR 序列暂不支持 FFmpeg 转码，已自动禁用。' : '开启后会在渲染完成时自动提交转码。' }}
                        </p>
                      </div>
                      <USwitch v-model="form.auto_transcode_mp4" color="primary" :disabled="isExrOutput" />
                    </div>
                    <div v-if="form.auto_transcode_mp4 && !isExrOutput" class="job-form-transcode-actions">
                      <p class="job-form-transcode-summary">{{ addJobTranscodeSummary }}</p>
                      <UButton
                        type="button"
                        icon="i-lucide-sliders-horizontal"
                        label="转码设置"
                        color="neutral"
                        variant="outline"
                        @click="addJobTranscodeSettingsOpen = true"
                      />
                    </div>
                  </div>
                </UFormField>
              </div>
              <p v-else class="hint-text">
                未检测到 Blender，请前往
                <UButton type="button" color="neutral" variant="link" size="sm" label="设置" @click="goToSettings" />
                添加。
              </p>
            </section>

            <section class="surface-panel transcode-submit-section">
              <div class="transcode-submit-head">
                <div>
                  <p class="choice-card-mode">检查结果</p>
                  <h2 class="choice-card-title">工程参数</h2>
                </div>
              </div>

              <div class="job-form-fields">
                <div class="job-form-inline-actions">
                  <UButton
                    type="button"
                    color="neutral"
                    variant="outline"
                    icon="i-lucide-search"
                    :loading="inspectingProject"
                    :disabled="!canInspectProject"
                    :label="inspectingProject ? '读取中…' : '读取工程参数'"
                    @click="inspectProjectSettings(true)"
                  />
                </div>

                <UAlert v-if="projectSettingsMessage" color="neutral" variant="subtle" :description="projectSettingsMessage" />

                <div v-if="projectSettings" class="job-form-stats">
                  <div class="job-form-stat">
                    <span class="job-form-stat-label">渲染引擎</span>
                    <strong>{{ displayEngine(projectSettings.engine) }}</strong>
                  </div>
                  <div class="job-form-stat">
                    <span class="job-form-stat-label">工程帧段</span>
                    <strong>{{ projectSettings.frameStart }}–{{ projectSettings.frameEnd }}</strong>
                  </div>
                  <div class="job-form-stat">
                    <span class="job-form-stat-label">分辨率</span>
                    <strong>{{ projectSettings.resolutionX }}×{{ projectSettings.resolutionY }}</strong>
                  </div>
                  <div class="job-form-stat">
                    <span class="job-form-stat-label">FPS</span>
                    <strong>{{ projectSettings.fps.toFixed(1) }}</strong>
                  </div>
                </div>
                <div v-else class="job-form-empty">
                  <UIcon name="i-lucide-scan-search" class="job-form-empty-icon" />
                  <p>点击“读取工程参数”后，这里会显示 Blender 工程内的渲染信息。</p>
                </div>
              </div>
            </section>

            <section v-if="notices.length" class="surface-panel transcode-submit-section">
              <div class="transcode-submit-head">
                <div>
                  <p class="choice-card-mode">辅助信息</p>
                  <h2 class="choice-card-title">提示</h2>
                </div>
              </div>

              <div class="notices-area notices-stack">
                <UAlert
                  v-for="(n, i) in notices"
                  :key="i"
                  :color="n.type === 'warn' ? 'warning' : 'neutral'"
                  variant="subtle"
                  :description="n.text"
                />
              </div>
            </section>
          </div>

          <UAlert v-if="formError" color="error" variant="subtle" :description="formError" />

          <div class="modal-actions settings-modal-actions">
            <div class="settings-modal-actions-start" />
            <div class="settings-modal-actions-end">
              <UButton type="button" icon="i-lucide-x" label="取消" color="neutral" variant="outline" @click="handleCloseAddJob" />
              <UButton
                type="submit"
                icon="i-lucide-plus"
                color="primary"
                variant="solid"
                :loading="submitting"
                :label="submitting ? '提交中…' : '加入渲染队列'"
              />
            </div>
          </div>
        </form>
      </template>
    </UModal>

    <TranscodeSubmitModal
      :open="addJobTranscodeSettingsOpen"
      mode="settings"
      :initial-config="addJobEffectiveTranscodeConfig"
      :base-config="addJobBaseTranscodeConfig"
      :folder-name="form.name || inferJobName(form.blend_file) || 'render'"
      :blender-job-name="form.name || inferJobName(form.blend_file) || 'render'"
      :blender-job-blend-file="form.blend_file"
      :blender-job-fps="projectSettings?.fps ?? null"
      :blender-job-output-path="form.output_path"
      @save-settings="handleAddJobTranscodeSettingsSave"
      @close="addJobTranscodeSettingsOpen = false"
      @update:open="addJobTranscodeSettingsOpen = $event"
    />

    <BlenderOutputSettingsModal
      v-model:open="addJobOutputSettingsOpen"
    />

    <UModal
      :open="showAddJobConfirm"
      :close="false"
      title="选择渲染方式"
      :ui="{ content: 'job-modal-content' }"
      @update:open="v => { if (!v) closeAddJobConfirm() }"
    >
      <template #body>
        <div class="modal-stack">
          <template v-if="addJobConflictKind === 'file'">
            <p class="modal-copy">
              检测到输出文件已存在：
              <strong>{{ addJobExistingOutputPath }}</strong>
            </p>
            <div class="choice-grid">
              <UCard variant="subtle" class="choice-card" :ui="{ body: 'choice-card-body' }">
                <div class="choice-card-head">
                  <p class="choice-card-mode">快速 MP4</p>
                  <h3 class="choice-card-title">覆盖现有文件并重新渲染</h3>
                </div>
                <p class="choice-card-desc">
                  该模式会直接输出单个 MP4 文件，不支持续跑或按区间补渲。
                </p>
                <UButton
                  color="neutral"
                  variant="outline"
                  label="覆盖并渲染"
                  class="choice-card-action"
                  :loading="submitting && addJobSubmitMode === 'restart'"
                  :disabled="submitting"
                  @click="submitPreparedJob(false)"
                />
              </UCard>
            </div>
          </template>
          <template v-else>
            <p class="modal-copy">
              检测到当前输出范围已存在
              <strong>{{ addJobFrameStatus?.frameCount ?? 0 }} 帧</strong>
              <template v-if="addJobFrameStatus?.lastFrame != null">
                ，当前最后一帧为 <strong>{{ addJobFrameStatus.lastFrame }}</strong>
              </template>
              。
            </p>
            <div class="choice-grid">
              <UCard variant="subtle" class="choice-card" :ui="{ body: 'choice-card-body' }">
                <div class="choice-card-head">
                  <p class="choice-card-mode">覆盖模式</p>
                  <h3 class="choice-card-title">重新开始渲染</h3>
                </div>
                <p class="choice-card-desc">
                  从第 <span class="choice-card-accent">{{ form.frame_start }}</span> 帧开始渲染，直接覆盖
                  <span class="choice-card-accent">{{ form.frame_start }}–{{ form.frame_end }}</span>
                  范围内的同名帧
                </p>
                <UButton
                  color="neutral"
                  variant="outline"
                  label="从头覆盖"
                  class="choice-card-action"
                  :loading="submitting && addJobSubmitMode === 'restart'"
                  :disabled="submitting"
                  @click="submitPreparedJob(false)"
                />
              </UCard>

              <UCard variant="subtle" class="choice-card" :ui="{ body: 'choice-card-body' }">
                <div class="choice-card-head">
                  <p class="choice-card-mode">续跑模式</p>
                  <h3 class="choice-card-title">从最后一帧继续</h3>
                </div>
                <p class="choice-card-desc">
                  <template v-if="addJobFrameStatus && addJobFrameStatus.nextFrame <= form.frame_end">
                    从第 <span class="choice-card-accent">{{ addJobFrameStatus.nextFrame }}</span> 帧继续渲染
                  </template>
                  <template v-else>
                    当前帧段已完整存在，继续将直接完成
                  </template>
                </p>
                <UButton
                  color="neutral"
                  variant="outline"
                  label="继续渲染"
                  class="choice-card-action"
                  :loading="submitting && addJobSubmitMode === 'continue'"
                  :disabled="submitting"
                  @click="submitPreparedJob(true)"
                />
              </UCard>
            </div>
          </template>
          <div class="modal-actions">
            <UButton icon="i-lucide-x" label="取消" color="warning" variant="outline" :disabled="submitting" @click="closeAddJobConfirm" />
          </div>
        </div>
      </template>
    </UModal>
  </div>
</template>

<script setup lang="ts">
import { open } from '@tauri-apps/plugin-dialog'
import { getCurrentWindow } from '@tauri-apps/api/window'
import type { DropdownMenuItem, TabsItem } from '@nuxt/ui'
import { useRouter } from 'vue-router'
import type { AddJobPayload, BlendProjectSettings, OutputPathTemplatePreview, RenderJob, RenderJobTranscodeConfig, RenderMode, RenderedFramesStatus } from '~/types'
import { buildTranscodeOutputPath, normalizeTranscodeDirectory, sanitizeTranscodeStemPart, splitTranscodeOutputPath } from '~/composables/useTranscodeConfig'

const router = useRouter()
const route = useRoute()
const jobsStore = useJobsStore()
const settingsStore = useSettingsStore()
const toast = useToast()
const activeQueueTab = ref<'all' | 'queue' | 'done' | 'error'>('all')

const initializingTools = ref(false)
const clearingCompletedRenderJobs = ref(false)
const showClearCompletedConfirm = ref(false)

async function initializeTools(options?: { silent?: boolean }) {
  const silent = options?.silent ?? false
  if (initializingTools.value) return
  initializingTools.value = true
  try {
    const toolchain = await inspectToolchain()
    const hadDefaultBlender = Boolean(settingsStore.settings.defaultBlender)
    await settingsStore.refreshBlenderVersions()
    const blenderAutoFilled = !hadDefaultBlender && Boolean(settingsStore.settings.defaultBlender)

    let ffmpegAutoFilled = false
    if (
      !settingsStore.settings.ffmpegExecutable
      && toolchain.ffmpegFound
      && toolchain.ffmpegExecutable
    ) {
      settingsStore.settings.ffmpegExecutable = toolchain.ffmpegExecutable
      ffmpegAutoFilled = true
    }
    if (blenderAutoFilled || ffmpegAutoFilled) {
      await settingsStore.save()
    }

    const blenderCount = toolchain.blenderInstalls.length

    if (silent) {
      return
    }

    if (blenderCount > 0 && toolchain.ffmpegFound) {
      const autoFilledDescription = blenderAutoFilled && ffmpegAutoFilled
        ? '已自动补全 Blender 和 FFmpeg 路径。'
        : blenderAutoFilled
          ? '已自动设置默认 Blender。'
          : ffmpegAutoFilled
            ? '已自动补全 FFmpeg 路径。'
            : 'Blender 和 FFmpeg 都可用。'
      toast.add({
        title: '渲染工具已就绪',
        description: autoFilledDescription,
        color: 'success',
        icon: 'i-lucide-check',
      })
    } else if (blenderCount > 0) {
      toast.add({
        title: '已找到 Blender',
        description: '未找到 FFmpeg，可在设置页补充路径。',
        color: 'warning',
        icon: 'i-lucide-triangle-alert',
      })
    } else if (toolchain.ffmpegFound) {
      toast.add({
        title: '已找到 FFmpeg',
        description: '未找到 Blender，请在设置页添加 Blender 路径。',
        color: 'warning',
        icon: 'i-lucide-triangle-alert',
      })
    } else {
      toast.add({
        title: '需要配置渲染工具',
        description: '未找到 Blender 和 FFmpeg，请前往设置页补充路径。',
        color: 'warning',
        icon: 'i-lucide-triangle-alert',
      })
    }
  } catch {
    if (!silent) {
      toast.add({
        title: '工具初始化失败',
        description: '请检查设置页中的工具路径。',
        color: 'error',
        icon: 'i-lucide-circle-alert',
      })
    }
  } finally {
    initializingTools.value = false
  }
}

async function handleStartQueue() {
  const hadPausedJob = !!jobsStore.pausedJobId
  try {
    await jobsStore.startQueue()
    toast.add({
      title: '队列已开始',
      description: hadPausedJob
        ? '已恢复被暂停的任务，将从断点继续渲染。'
        : '等待中的任务会按当前队列顺序依次启动。',
      color: 'success',
    })
  } catch (error) {
    toast.add({
      title: '开始失败',
      description: error instanceof Error ? error.message : String(error),
      color: 'error',
    })
  }
}

async function handlePauseQueue() {
  try {
    await jobsStore.pauseQueue()
    toast.add({
      title: '队列已暂停',
      description: '当前渲染任务已中止，点击“开始任务队列”可从断点自动续跑。',
      color: 'warning',
    })
  } catch (error) {
    toast.add({
      title: '暂停失败',
      description: error instanceof Error ? error.message : String(error),
      color: 'error',
    })
  }
}

function handleQueueToggle() {
  return jobsStore.queuePaused ? handleStartQueue() : handlePauseQueue()
}

const showInitializeTools = computed(() => {
  const hasBlender = Boolean(settingsStore.settings.defaultBlender)
  const hasFfmpeg = Boolean(settingsStore.settings.ffmpegExecutable)
  return !(hasBlender && hasFfmpeg)
})

const queueToggleDisabled = computed(() =>
  jobsStore.queuePaused
    ? (!jobsStore.pendingJobs.length && !jobsStore.pausedJobId)
    : false,
)

const queueToggleButton = computed(() => ({
  icon: jobsStore.queuePaused ? 'i-lucide-play' : 'i-lucide-pause',
  label: jobsStore.queuePaused ? '开始任务队列' : '暂停任务队列',
  color: jobsStore.queuePaused ? 'success' as const : 'warning' as const,
}))

const queueToggleTooltip = computed(() =>
  jobsStore.queuePaused
    ? '按当前顺序启动等待中的任务'
    : '立即中止当前渲染任务并暂停队列，恢复时会从断点自动续跑',
)

const { validateBlendFile, inspectBlendFile, inspectRenderedFrames, inspectToolchain, previewOutputPathTemplate, pathExists } = useTauri()

const deleteConfirmJob = ref<RenderJob | null>(null)
const {
  showRetryConfirm,
  retryConfirmJob,
  retryIsQuickMp4,
  retryActionError,
  retrySubmittingMode,
  retryCustomStart,
  retryCustomEnd,
  retryFullRangePreviewMode,
  retryCustomPreviewMode,
  retryCustomInspectLoading,
  retryAutoTranscodeEnabled,
  retryTranscodeRangeMode,
  retryFullRangeLabel,
  retryFullRangeSummary,
  retryCustomActionDescription,
  retryCustomRangeSummary,
  retryOriginalTranscodeRangeLabel,
  retrySavedTranscodeRangeTitle,
  retryCurrentTargetRangeLabel,
  retryTranscodeSummary,
  cancelRetryCloseCleanup,
  handleRetry,
  closeRetryConfirm,
  clearRetryPreviewOnLeave,
  confirmRetryContinue,
  confirmRetryFromStart,
  confirmRetryCustomRange,
} = useJobRetry()
const draggedJobId = ref<string | null>(null)
const dropTargetJobId = ref<string | null>(null)
const dropPosition = ref<'before' | 'after'>('before')
const reorderingQueue = ref(false)
const pointerDragging = ref(false)
let dragPointerId: number | null = null
let dragStartX = 0
let dragStartY = 0
let pendingDragJobId: string | null = null
const DRAG_START_DISTANCE = 6

function confirmDelete() {
  if (deleteConfirmJob.value) jobsStore.deleteJob(deleteConfirmJob.value.id)
  deleteConfirmJob.value = null
}

const isDragging = ref(false)
let unlistenDrop: (() => void) | null = null

onMounted(async () => {
  await settingsStore.load()
  if (showInitializeTools.value) {
    await initializeTools()
  }
  unlistenDrop = await getCurrentWindow().onDragDropEvent((event) => {
    if (route.path !== '/') return
    if (draggedJobId.value) return
    if (event.payload.type === 'enter' || event.payload.type === 'over') {
      isDragging.value = true
    } else if (event.payload.type === 'leave') {
      isDragging.value = false
    } else if (event.payload.type === 'drop') {
      isDragging.value = false
      const blendPath = (event.payload.paths as string[]).find(p => p.toLowerCase().endsWith('.blend'))
      if (blendPath) openAddJobWithFile(blendPath).catch(console.error)
    }
  })
})

const showAddJob = ref(false)
const showAddJobConfirm = ref(false)
const addJobTranscodeSettingsOpen = ref(false)
const metadataDialogOpen = ref(false)
const metadataJob = ref<RenderJob | null>(null)
const submitting = ref(false)
const addJobSubmitMode = ref<'restart' | 'continue' | null>(null)
const inspectingProject = ref(false)
const formError = ref('')
const projectSettings = ref<BlendProjectSettings | null>(null)
const projectSettingsMessage = ref('')
const outputFrameStatus = ref<RenderedFramesStatus | null>(null)
const addJobFrameStatus = ref<RenderedFramesStatus | null>(null)
const addJobConflictKind = ref<'frames' | 'file' | null>(null)
const addJobExistingOutputPath = ref('')
const outputPathPreview = ref<OutputPathTemplatePreview | null>(null)
const resolvedAddJobBaseTranscodeOutputPath = ref('')
let inspectTimer: ReturnType<typeof setTimeout> | null = null
let outputDirTimer: ReturnType<typeof setTimeout> | null = null
let outputPathPreviewTimer: ReturnType<typeof setTimeout> | null = null


const ENGINE_NAMES: Record<string, string> = {
  BLENDER_EEVEE: 'EEVEE',
  BLENDER_EEVEE_NEXT: 'EEVEE Next',
  CYCLES: 'Cycles',
  BLENDER_WORKBENCH: 'Workbench',
}
function displayEngine(engine: string) {
  return ENGINE_NAMES[engine] ?? engine
}

type OutputMode = 'PNG' | 'OPEN_EXR' | 'QUICK_MP4'

const QUICK_MP4_OUTPUT_PATH_TEMPLATE = './快速预览/{blendFileName}_{frameStart}-{frameEnd}.mp4'
const SEQUENCE_FORMATS = ['PNG', 'OPEN_EXR']
const outputFormatOptions = [
  { label: 'PNG 序列', value: 'PNG' },
  { label: 'OpenEXR 序列', value: 'OPEN_EXR' },
  { label: '快速 MP4', value: 'QUICK_MP4' },
] satisfies Array<{ label: string, value: OutputMode }>
const outputModeLabelMap: Record<OutputMode, string> = {
  PNG: 'PNG 序列',
  OPEN_EXR: 'OpenEXR 序列',
  QUICK_MP4: '快速 MP4',
}
const queueTabItems = computed<TabsItem[]>(() => [
  { label: '全部', value: 'all', badge: { label: String(jobsStore.jobs.length), color: 'neutral' as const, variant: 'subtle' as const }, icon: 'i-lucide-layers', class: 'queue-tab-tone-all', ui: { trigger: 'queue-tab-tone-all' } },
  { label: '排队中', value: 'queue', badge: { label: String(jobsStore.queueJobs.length), color: 'info' as const, variant: 'subtle' as const }, icon: 'i-lucide-loader-circle', class: 'queue-tab-tone-queue', ui: { trigger: 'queue-tab-tone-queue' } },
  { label: '已完成', value: 'done', badge: { label: String(jobsStore.doneJobs.length), color: 'success' as const, variant: 'subtle' as const }, icon: 'i-lucide-circle-check-big', class: 'queue-tab-tone-done', ui: { trigger: 'queue-tab-tone-done' } },
  { label: '已中止', value: 'error', badge: { label: String(jobsStore.errorJobs.length), color: 'warning' as const, variant: 'subtle' as const }, icon: 'i-lucide-triangle-alert', class: 'queue-tab-tone-error', ui: { trigger: 'queue-tab-tone-error' } },
])
const filteredJobs = computed(() => {
  switch (activeQueueTab.value) {
    case 'queue':
      return jobsStore.queueJobs
    case 'done':
      return jobsStore.doneJobs
    case 'error':
      return jobsStore.errorJobs
    default:
      return jobsStore.jobs
  }
})
const emptyTabTitle = computed(() => {
  switch (activeQueueTab.value) {
    case 'queue':
      return '当前没有排队中任务'
    case 'done':
      return '当前没有已完成任务'
    case 'error':
      return '当前没有已中止任务'
    default:
      return '当前没有任务'
  }
})
const emptyTabNote = computed(() => {
  switch (activeQueueTab.value) {
    case 'queue':
      return '新建任务后会先进入队列，点击上方“开始任务”后再按顺序执行。'
    case 'done':
      return '完成的任务会显示在这里，方便单独查看已结束项目。'
    case 'error':
      return '失败、已取消和已中断的任务会集中显示在这里。'
    default:
      return '这里会显示当前筛选下的任务卡片。'
  }
})
const emptyTabToneClass = computed(() => {
  switch (activeQueueTab.value) {
    case 'queue':
      return 'queue-empty-tone-queue'
    case 'done':
      return 'queue-empty-tone-done'
    case 'error':
      return 'queue-empty-tone-error'
    default:
      return 'queue-empty-tone-all'
  }
})
const renderQueueActionItems = computed<DropdownMenuItem[][]>(() => [[
  {
    label: '清理已完成',
    icon: 'i-lucide-trash-2',
    disabled: clearingCompletedRenderJobs.value || jobsStore.doneJobs.length === 0,
    onSelect: () => {
      showClearCompletedConfirm.value = true
    },
  },
]])

function canDragQueueJob(job: RenderJob) {
  return job.status !== 'running' && !reorderingQueue.value
}

async function confirmClearCompletedRenderJobs() {
  if (clearingCompletedRenderJobs.value || jobsStore.doneJobs.length === 0) return

  clearingCompletedRenderJobs.value = true
  try {
    const { removed, failed } = await jobsStore.clearCompletedJobs()
    if (removed > 0) {
      toast.add({
        title: '已清理完成任务',
        description: failed > 0 ? `成功清理 ${removed} 个，另有 ${failed} 个失败。` : `成功清理 ${removed} 个已完成任务。`,
        color: failed > 0 ? 'warning' : 'success',
      })
    } else if (failed > 0) {
      toast.add({
        title: '清理完成任务失败',
        description: `共有 ${failed} 个任务未能删除。`,
        color: 'error',
      })
    }
  } finally {
    showClearCompletedConfirm.value = false
    clearingCompletedRenderJobs.value = false
  }
}

function clearQueueDragState() {
  pointerDragging.value = false
  draggedJobId.value = null
  dropTargetJobId.value = null
  dropPosition.value = 'before'
  dragPointerId = null
  pendingDragJobId = null
  document.body.style.removeProperty('user-select')
}

function updateQueueDropPosition(jobId: string, target: HTMLElement, clientY: number) {
  if (!draggedJobId.value || draggedJobId.value === jobId) return
  const job = jobsStore.jobs.find(item => item.id === jobId)
  if (!job || !canDragQueueJob(job)) return
  if (!target) return
  const rect = target.getBoundingClientRect()
  dropTargetJobId.value = jobId
  dropPosition.value = clientY >= rect.top + rect.height / 2 ? 'after' : 'before'
}

function beginQueuePointerDrag(jobId: string) {
  pointerDragging.value = true
  draggedJobId.value = jobId
  dropTargetJobId.value = null
  dropPosition.value = 'before'
  document.body.style.userSelect = 'none'
}

function handleQueuePointerMove(event: PointerEvent) {
  if (dragPointerId == null || event.pointerId !== dragPointerId || !pendingDragJobId) return

  if (!pointerDragging.value) {
    const movedX = Math.abs(event.clientX - dragStartX)
    const movedY = Math.abs(event.clientY - dragStartY)
    if (Math.max(movedX, movedY) < DRAG_START_DISTANCE) {
      return
    }
    beginQueuePointerDrag(pendingDragJobId)
  }

  const target = document.elementFromPoint(event.clientX, event.clientY)?.closest('.job-list-item') as HTMLElement | null
  const targetJobId = target?.dataset.jobId
  if (!target || !targetJobId) return
  updateQueueDropPosition(targetJobId, target, event.clientY)
}

async function commitQueuePointerDrop() {
  if (!draggedJobId.value || !dropTargetJobId.value || draggedJobId.value === dropTargetJobId.value) {
    clearQueueDragState()
    return
  }

  const draggedId = draggedJobId.value
  const queueIds = jobsStore.jobs
    .filter(queueJob => queueJob.status !== 'running')
    .map(queueJob => queueJob.id)
  if (!queueIds.includes(draggedId) || !queueIds.includes(dropTargetJobId.value)) {
    clearQueueDragState()
    return
  }

  const remainingIds = queueIds.filter(id => id !== draggedId)
  const targetIndex = remainingIds.indexOf(dropTargetJobId.value)
  if (targetIndex === -1) {
    clearQueueDragState()
    return
  }

  const insertIndex = dropPosition.value === 'after' ? targetIndex + 1 : targetIndex
  remainingIds.splice(insertIndex, 0, draggedId)

  reorderingQueue.value = true
  try {
    await jobsStore.reorderQueueJobs(remainingIds)
  } catch (error) {
    toast.add({
      title: '顺序更新失败',
      description: error instanceof Error ? error.message : String(error),
      color: 'error',
    })
  } finally {
    reorderingQueue.value = false
    clearQueueDragState()
  }
}

function handleQueuePointerUp(event: PointerEvent) {
  if (dragPointerId == null || event.pointerId !== dragPointerId) return
  window.removeEventListener('pointermove', handleQueuePointerMove)
  window.removeEventListener('pointerup', handleQueuePointerUp)
  window.removeEventListener('pointercancel', handleQueuePointerCancel)
  if (!pointerDragging.value) {
    clearQueueDragState()
    return
  }
  void commitQueuePointerDrop()
}

function handleQueuePointerCancel() {
  window.removeEventListener('pointermove', handleQueuePointerMove)
  window.removeEventListener('pointerup', handleQueuePointerUp)
  window.removeEventListener('pointercancel', handleQueuePointerCancel)
  clearQueueDragState()
}

function handleQueuePointerDown(job: RenderJob, event: PointerEvent) {
  if (event.button !== 0) return
  if (!canDragQueueJob(job) || reorderingQueue.value) return
  const target = event.target as HTMLElement | null
  if (target?.closest('button, a, input, textarea, select, [data-no-drag], [role="menuitem"], [role="checkbox"], [contenteditable="true"]')) {
    return
  }

  dragPointerId = event.pointerId
  pendingDragJobId = job.id
  dragStartX = event.clientX
  dragStartY = event.clientY
  window.addEventListener('pointermove', handleQueuePointerMove)
  window.addEventListener('pointerup', handleQueuePointerUp)
  window.addEventListener('pointercancel', handleQueuePointerCancel)
}

async function autoFillOutputPath(blendPath: string) {
  const name = (inferJobName(blendPath) || 'render').replace(/[<>:"/\\|?*]/g, '_')
  if (!form.name) form.name = name

  try {
    const preview = await previewOutputPathTemplate({
      kind: 'blender',
      template: isQuickMp4Output.value
        ? QUICK_MP4_OUTPUT_PATH_TEMPLATE
        : settingsStore.settings.renderOutputPathTemplate,
      blend_file: blendPath,
      frame_start: form.frame_start,
      frame_end: form.frame_end,
    })
    form.output_path = preview.resolvedPath || (
      isQuickMp4Output.value
        ? QUICK_MP4_OUTPUT_PATH_TEMPLATE
        : settingsStore.settings.renderOutputPathTemplate
    )
  } catch {
    form.output_path = isQuickMp4Output.value
      ? QUICK_MP4_OUTPUT_PATH_TEMPLATE
      : settingsStore.settings.renderOutputPathTemplate
  }
}

function isAutoResolvedRenderOutputPath(path: string) {
  const trimmed = path.trim()
  if (!trimmed || !form.blend_file) return false
  return trimmed === (outputPathPreview.value?.resolvedPath ?? '')
}

function buildOutputPatternForDirectory(directory: string) {
  const normalizedDirectory = directory.replace(/[\\/]+$/, '')
  if (!normalizedDirectory) return ''

  const defaultName = (form.name || inferJobName(form.blend_file) || 'render').replace(/[<>:"/\\|?*]/g, '_')
  if (isQuickMp4Output.value) {
    const quickName = (inferJobName(form.blend_file) || form.name || 'render').replace(/[<>:"/\\|?*]/g, '_')
    return `${normalizedDirectory}\\${quickName}_${form.frame_start}-${form.frame_end}.mp4`
  }

  const currentPattern = form.output_path.split(/[/\\]/).pop() || ''
  const outputPattern = currentPattern.includes('#') || currentPattern.includes('{')
    ? currentPattern
    : `${defaultName}_######`
  const separator = normalizedDirectory.includes('\\') && !normalizedDirectory.includes('/') ? '\\' : '/'
  return `${normalizedDirectory}${separator}${outputPattern}`
}

async function browseBlendFile() {
  const selected = await open({
    multiple: false,
    directory: false,
    title: '选择 Blend 文件',
    defaultPath: form.blend_file || undefined,
    filters: [{ name: 'Blender Project', extensions: ['blend'] }],
  })

  if (typeof selected !== 'string' || !selected) return

  const previousBlendFile = form.blend_file
  form.blend_file = selected

  if (!form.name || (previousBlendFile && form.name === inferJobName(previousBlendFile))) {
    form.name = inferJobName(selected)
  }

  if (!form.output_path) {
    await autoFillOutputPath(selected)
  }
}

async function browseRenderOutputDirectory() {
  const defaultDirectory = form.output_path
    ? form.output_path.replace(/[/\\][^/\\]*$/, '')
    : undefined
  const selected = await open({
    directory: true,
    multiple: false,
    title: '选择输出目录',
    defaultPath: defaultDirectory,
  })

  if (typeof selected !== 'string' || !selected) return
  form.output_path = buildOutputPatternForDirectory(selected)
}

const form = reactive<AddJobPayload>({
  name: '',
  note: '',
  render_mode: 'image_sequence',
  auto_transcode_mp4: false,
  transcode_name_override: null,
  transcode_fps_override: null,
  transcode_output_path_override: null,
  transcode_crf_override: null,
  transcode_preset_override: null,
  blend_file: '',
  blender_executable: '',
  output_path: '',
  output_format: 'PNG',
  frame_start: 1,
  frame_end: 250,
  resume_from_existing: true,
  priority: 0,
})
const addJobOutputSettingsOpen = ref(false)

const formNote = computed({
  get: () => form.note ?? '',
  set: (value: string) => {
    form.note = value
  },
})

function resetForm() {
  form.name = ''
  form.note = ''
  form.render_mode = 'image_sequence'
  form.auto_transcode_mp4 = false
  form.transcode_name_override = null
  form.transcode_fps_override = null
  form.transcode_output_path_override = null
  form.transcode_crf_override = null
  form.transcode_preset_override = null
  form.blend_file = ''
  form.blender_executable = settingsStore.settings.defaultBlender || settingsStore.blenderVersions[0]?.executable || ''
  form.output_path = ''
  form.output_format = 'PNG'
  form.frame_start = 1
  form.frame_end = 250
  form.resume_from_existing = true
  form.priority = 0
  formError.value = ''
  projectSettings.value = null
  projectSettingsMessage.value = ''
  outputFrameStatus.value = null
  addJobFrameStatus.value = null
  addJobConflictKind.value = null
  addJobExistingOutputPath.value = ''
  outputPathPreview.value = null
  resolvedAddJobBaseTranscodeOutputPath.value = ''
  addJobTranscodeSettingsOpen.value = false
}

async function ensureSettingsLoaded() {
  if (!settingsStore.blenderVersions.length && !settingsStore.settings.defaultBlender) {
    await settingsStore.load()
  }
}

async function openAddJob() {
  await ensureSettingsLoaded()
  resetForm()
  showAddJob.value = true
}

function openMetadataDialog(job: RenderJob) {
  metadataJob.value = job
  metadataDialogOpen.value = true
}

function buildJobContextMenuItems(job: RenderJob) {
  return [
    {
      label: '编辑项目信息',
      icon: 'i-lucide-notebook-pen',
      onSelect: () => openMetadataDialog(job),
    },
  ]
}

async function openAddJobWithFile(blendPath: string) {
  await ensureSettingsLoaded()
  resetForm()
  form.blend_file = blendPath
  await autoFillOutputPath(blendPath)
  showAddJob.value = true
}

function closeAddJob(force = false) {
  if (submitting.value && !force) return
  showAddJob.value = false
  showAddJobConfirm.value = false
  formError.value = ''
  projectSettings.value = null
  projectSettingsMessage.value = ''
  outputFrameStatus.value = null
  addJobFrameStatus.value = null
  addJobConflictKind.value = null
  addJobExistingOutputPath.value = ''
  outputPathPreview.value = null
  resolvedAddJobBaseTranscodeOutputPath.value = ''
}

function handleCloseAddJob() {
  closeAddJob()
}

function closeAddJobConfirm() {
  if (submitting.value) return
  showAddJobConfirm.value = false
  addJobFrameStatus.value = null
  addJobConflictKind.value = null
  addJobExistingOutputPath.value = ''
}

function goToSettings() {
      closeAddJob(true)
  router.push('/settings')
}

function inferJobName(path: string) {
  const filename = path.split(/[/\\]/).pop() || ''
  return filename.replace(/\.blend$/i, '')
}

function selectBlender(executable: string) {
  form.blender_executable = executable
}

function deriveAddJobSequenceDirectory(outputPath: string) {
  const normalized = outputPath.replace(/\\/g, '/')
  const slashIndex = normalized.lastIndexOf('/')
  if (normalized.includes('#') || normalized.includes('{frame}')) {
    return slashIndex >= 0 ? outputPath.slice(0, slashIndex) : outputPath
  }
  return slashIndex >= 0 ? outputPath.slice(0, slashIndex) : outputPath
}

async function refreshAddJobBaseTranscodeOutputPath() {
  if (!form.blend_file) {
    resolvedAddJobBaseTranscodeOutputPath.value = ''
    return
  }

  try {
    const preview = await previewOutputPathTemplate({
      kind: 'blender-ffmpeg',
      template: settingsStore.settings.blenderTranscodeOutputPathTemplate,
      blend_file: form.blend_file,
      frame_start: form.frame_start,
      frame_end: form.frame_end,
    })
    resolvedAddJobBaseTranscodeOutputPath.value = preview.resolvedPath || ''
  } catch {
    resolvedAddJobBaseTranscodeOutputPath.value = ''
  }
}

const addJobBaseTranscodeConfig = computed<RenderJobTranscodeConfig>(() => {
  const renderName = form.name.trim() || inferJobName(form.blend_file) || 'render'
  const fallbackDir = normalizeTranscodeDirectory(deriveAddJobSequenceDirectory(form.output_path))
  const fallbackOutputPath = buildTranscodeOutputPath(fallbackDir, sanitizeTranscodeStemPart(renderName))
  const outputPath = resolvedAddJobBaseTranscodeOutputPath.value || fallbackOutputPath
  const split = splitTranscodeOutputPath(outputPath)

  return {
    name: renderName,
    fps: Math.max(1, Math.round(projectSettings.value?.fps && projectSettings.value.fps > 0 ? projectSettings.value.fps : 30)),
    outputPath,
    outputDir: split.outputDir,
    outputStem: split.outputStem,
    crf: settingsStore.settings.transcodeCrf,
    preset: settingsStore.settings.transcodePreset,
  }
})

const addJobEffectiveTranscodeConfig = computed<RenderJobTranscodeConfig>(() => {
  const base = addJobBaseTranscodeConfig.value
  const outputPath = form.transcode_output_path_override || base.outputPath
  const split = splitTranscodeOutputPath(outputPath)

  return {
    name: form.transcode_name_override || base.name,
    fps: Math.max(1, Math.round(form.transcode_fps_override && form.transcode_fps_override > 0 ? form.transcode_fps_override : base.fps)),
    outputPath,
    outputDir: split.outputDir,
    outputStem: split.outputStem,
    crf: form.transcode_crf_override ?? base.crf,
    preset: form.transcode_preset_override || base.preset,
  }
})

const selectedOutputMode = computed<OutputMode>({
  get: () => {
    if (form.render_mode === 'quick_mp4') return 'QUICK_MP4'
    return form.output_format === 'OPEN_EXR' ? 'OPEN_EXR' : 'PNG'
  },
  set: (value) => {
    if (value === 'QUICK_MP4') {
      form.render_mode = 'quick_mp4'
      form.output_format = 'FFMPEG'
      form.auto_transcode_mp4 = false
      addJobTranscodeSettingsOpen.value = false
      return
    }

    form.render_mode = 'image_sequence'
    form.output_format = value
  },
})

const addJobTranscodeSummary = computed(() => {
  const config = addJobEffectiveTranscodeConfig.value
  return `${config.outputStem}.mp4 · ${config.fps} FPS · CRF ${config.crf} · ${config.preset}`
})

const isQuickMp4Output = computed(() => form.render_mode === 'quick_mp4')
const outputPathPlaceholder = computed(() => (
  isQuickMp4Output.value
    ? 'F:\\项目\\快速预览\\潜行瞬鲨_1-250.mp4'
    : 'F:\\项目\\潜行瞬鲨_1-250\\潜行瞬鲨_######'
))

const outputSettingsTitle = computed(() => (
  isQuickMp4Output.value ? '快速 MP4 固定预设' : '图像序列输出设置'
))

const outputSettingsSummary = computed(() => {
  if (isQuickMp4Output.value) {
    return 'Blender 直出 H.264 MP4 · 固定预设 · 不进入转码队列'
  }
  if (form.output_format === 'OPEN_EXR') {
    return [
      'OpenEXR',
      settingsStore.settings.exrColorMode,
      `${settingsStore.settings.exrColorDepth}-bit`,
      settingsStore.settings.exrCodec,
      `质量 ${settingsStore.settings.exrQuality}%`,
    ].join(' · ')
  }

  return [
    'PNG',
    settingsStore.settings.pngColorMode,
    `${settingsStore.settings.pngColorDepth}-bit`,
    `压缩 ${settingsStore.settings.pngCompression}`,
  ].join(' · ')
})

const isExrOutput = computed(() => form.output_format === 'OPEN_EXR')
const outputPathTemplateHasErrors = computed(() => Boolean(outputPathPreview.value?.errors.length))

const canInspectProject = computed(() => {
  return Boolean(form.blender_executable && form.blend_file && form.blend_file.toLowerCase().endsWith('.blend'))
})

const notices = computed(() => {
  const list: { type: 'warn' | 'info'; text: string }[] = []
  if (projectSettings.value?.outputFormat === 'FFMPEG') {
    list.push({
      type: 'warn',
      text: isQuickMp4Output.value
        ? '工程原输出为视频 (FFMPEG)，当前将直接输出 MP4。'
        : '工程原输出为视频 (FFMPEG)，将按所选图像格式渲染序列帧。',
    })
  }
  if (isQuickMp4Output.value && addJobExistingOutputPath.value) {
    list.push({ type: 'warn', text: `检测到目标 MP4 已存在：${addJobExistingOutputPath.value}` })
  }
  if ((outputFrameStatus.value?.frameCount ?? 0) > 0) {
    const suffix = outputFrameStatus.value?.lastFrame != null ? `，最后一帧 ${outputFrameStatus.value.lastFrame}` : ''
    list.push({ type: 'warn', text: `检测到当前帧段已存在 ${outputFrameStatus.value?.frameCount} 帧${suffix}，提交时会询问从头覆盖还是断点继续。` })
  }
  return list
})

async function inspectProjectSettings(showErrors = false) {
  if (!canInspectProject.value || inspectingProject.value) return
  inspectingProject.value = true

  try {
    const shouldRefreshAutoOutputPath = !form.output_path || isAutoResolvedRenderOutputPath(form.output_path)
    const settings = await inspectBlendFile(form.blender_executable, form.blend_file)
    projectSettings.value = settings
    projectSettingsMessage.value = '已从工程读取渲染参数。'
    form.frame_start = settings.frameStart
    form.frame_end = settings.frameEnd

    if (settings.outputFormat && SEQUENCE_FORMATS.includes(settings.outputFormat)) {
      form.output_format = settings.outputFormat
    }

    // Always default to blend-file-adjacent renders folder; ignore project's configured output path
    if (shouldRefreshAutoOutputPath && form.blend_file) {
      await autoFillOutputPath(form.blend_file)
    }
  } catch (error) {
    projectSettings.value = null
    if (showErrors) {
      projectSettingsMessage.value = error instanceof Error ? error.message : String(error)
    }
  } finally {
    inspectingProject.value = false
  }
}

async function checkOutputDir() {
  if (!outputPathPreview.value?.resolvedPath || outputPathTemplateHasErrors.value) {
    outputFrameStatus.value = null
    addJobExistingOutputPath.value = ''
    return
  }
  if (isQuickMp4Output.value) {
    outputFrameStatus.value = null
    addJobExistingOutputPath.value = await pathExists(outputPathPreview.value.resolvedPath)
      ? outputPathPreview.value.resolvedPath
      : ''
    return
  }
  try {
    outputFrameStatus.value = await inspectRenderedFrames(
      outputPathPreview.value.resolvedPath,
      form.output_format,
      form.frame_start,
      form.frame_end,
    )
  } catch {
    outputFrameStatus.value = null
  }
  addJobExistingOutputPath.value = ''
}

async function refreshOutputPathPreview() {
  const template = form.output_path.trim()
  if (!template) {
    outputPathPreview.value = null
    return
  }

  try {
    outputPathPreview.value = await previewOutputPathTemplate({
      kind: 'blender',
      template,
      blend_file: form.blend_file || null,
      frame_start: form.frame_start,
      frame_end: form.frame_end,
    })
  } catch (error) {
    outputPathPreview.value = {
      resolvedPath: null,
      errors: [error instanceof Error ? error.message : String(error)],
      notes: [],
    }
  }
}

watch(
  [() => form.blender_executable, () => form.blend_file],
  ([newExec, newBlend], [oldExec, oldBlend]) => {
    if (newExec === oldExec && newBlend === oldBlend) return

    if (newBlend !== oldBlend && newBlend.toLowerCase().endsWith('.blend') && !form.output_path) {
      void autoFillOutputPath(newBlend)
    }

    projectSettingsMessage.value = ''
    if (inspectTimer) { clearTimeout(inspectTimer); inspectTimer = null }
    projectSettings.value = null
    void refreshAddJobBaseTranscodeOutputPath()
    if (outputPathPreviewTimer) { clearTimeout(outputPathPreviewTimer); outputPathPreviewTimer = null }
    outputPathPreviewTimer = setTimeout(() => void refreshOutputPathPreview(), 120)
  },
)

watch(
  [() => form.output_path, () => form.output_format, () => form.frame_start, () => form.frame_end],
  () => {
    void refreshAddJobBaseTranscodeOutputPath()
    if (outputPathPreviewTimer) { clearTimeout(outputPathPreviewTimer); outputPathPreviewTimer = null }
    outputPathPreviewTimer = setTimeout(() => void refreshOutputPathPreview(), 250)
    if (outputDirTimer) { clearTimeout(outputDirTimer); outputDirTimer = null }
    outputDirTimer = setTimeout(() => void checkOutputDir(), 600)
  },
)

watch(
  () => settingsStore.settings.blenderTranscodeOutputPathTemplate,
  () => {
    void refreshAddJobBaseTranscodeOutputPath()
  },
)

watch(
  () => form.output_format,
  (value) => {
    if (value !== 'OPEN_EXR') return
    form.auto_transcode_mp4 = false
    addJobTranscodeSettingsOpen.value = false
  },
  { immediate: true },
)

watch(
  selectedOutputMode,
  () => {
    if (form.blend_file) {
      void autoFillOutputPath(form.blend_file)
    }
    void checkOutputDir()
  },
)

function buildJobPayload(resumeFromExisting: boolean): AddJobPayload {
  const seededFrameStatus = resumeFromExisting ? addJobFrameStatus.value : null
  const totalFrames = form.frame_end - form.frame_start + 1
  const autoTranscodeEnabled = form.auto_transcode_mp4 && !isExrOutput.value && !isQuickMp4Output.value
  return {
    ...form,
    render_mode: form.render_mode as RenderMode,
    auto_transcode_mp4: autoTranscodeEnabled,
    transcode_name_override: autoTranscodeEnabled ? form.transcode_name_override : null,
    transcode_fps_override: autoTranscodeEnabled ? form.transcode_fps_override : null,
    transcode_output_path_override: autoTranscodeEnabled ? form.transcode_output_path_override : null,
    transcode_crf_override: autoTranscodeEnabled ? form.transcode_crf_override : null,
    transcode_preset_override: autoTranscodeEnabled ? form.transcode_preset_override : null,
    name: form.name || inferJobName(form.blend_file) || 'Untitled Render Job',
    fps: projectSettings.value?.fps ?? null,
    preview_width: projectSettings.value?.resolutionX ?? null,
    preview_height: projectSettings.value?.resolutionY ?? null,
    output_format: isQuickMp4Output.value ? 'FFMPEG' : form.output_format,
    resume_from_existing: isQuickMp4Output.value ? false : resumeFromExisting,
    initial_current_frame: isQuickMp4Output.value ? null : (seededFrameStatus?.frameCount ?? null),
    initial_total_frames: isQuickMp4Output.value ? null : (seededFrameStatus ? totalFrames : null),
    initial_last_rendered_frame: isQuickMp4Output.value ? null : (seededFrameStatus?.lastFrame ?? null),
  }
}

function handleAddJobTranscodeSettingsSave(payload: {
  transcode_name_override: string | null
  transcode_fps_override: number | null
  transcode_output_path_override: string | null
  transcode_crf_override: number | null
  transcode_preset_override: string | null
}) {
  form.transcode_name_override = payload.transcode_name_override
  form.transcode_fps_override = payload.transcode_fps_override
  form.transcode_output_path_override = payload.transcode_output_path_override
  form.transcode_crf_override = payload.transcode_crf_override
  form.transcode_preset_override = payload.transcode_preset_override
  addJobTranscodeSettingsOpen.value = false
}

async function submitNewJob() {
  if (submitting.value) return
  formError.value = ''

  if (!form.blend_file || !form.blender_executable || !form.output_path) {
    formError.value = 'Blend 文件、Blender 可执行文件和输出路径不能为空。'
    return
  }
  if (outputPathTemplateHasErrors.value) {
    formError.value = outputPathPreview.value?.errors[0] || '输出路径模板无效。'
    return
  }

  if (form.frame_start > form.frame_end) {
    formError.value = '起始帧不能大于结束帧。'
    return
  }

  submitting.value = true

  try {
    await validateBlendFile(form.blend_file)
    const resolvedOutputPath = outputPathPreview.value?.resolvedPath
    if (!resolvedOutputPath) {
      formError.value = '当前输出路径还无法解析，请先检查模板变量。'
      return
    }
    if (isQuickMp4Output.value) {
      const exists = await pathExists(resolvedOutputPath)
      if (exists) {
        addJobConflictKind.value = 'file'
        addJobExistingOutputPath.value = resolvedOutputPath
        showAddJobConfirm.value = true
        return
      }
    }
    if (!isQuickMp4Output.value) {
      const status = await inspectRenderedFrames(resolvedOutputPath, form.output_format, form.frame_start, form.frame_end)
        .catch(() => ({ frameCount: 0, lastFrame: null, nextFrame: form.frame_start }))
      if (status.frameCount > 0) {
        addJobConflictKind.value = 'frames'
        addJobFrameStatus.value = status
        showAddJobConfirm.value = true
        return
      }
    }

    await jobsStore.submitJob(buildJobPayload(true))
    closeAddJob(true)
  } catch (error) {
    formError.value = error instanceof Error ? error.message : String(error)
  } finally {
    submitting.value = false
  }
}

async function submitPreparedJob(resumeFromExisting: boolean) {
  if (submitting.value) return
  formError.value = ''
  addJobSubmitMode.value = resumeFromExisting ? 'continue' : 'restart'
  submitting.value = true
  try {
    await jobsStore.submitJob(buildJobPayload(resumeFromExisting))
    closeAddJob(true)
  } catch (error) {
    formError.value = error instanceof Error ? error.message : String(error)
  } finally {
    addJobSubmitMode.value = null
    if (showAddJob.value) {
      closeAddJobConfirm()
    }
    submitting.value = false
  }
}

onUnmounted(() => {
  cancelRetryCloseCleanup()
  unlistenDrop?.()
  window.removeEventListener('pointermove', handleQueuePointerMove)
  window.removeEventListener('pointerup', handleQueuePointerUp)
  window.removeEventListener('pointercancel', handleQueuePointerCancel)
  document.body.style.removeProperty('user-select')
  if (inspectTimer) clearTimeout(inspectTimer)
  if (outputPathPreviewTimer) clearTimeout(outputPathPreviewTimer)
  if (outputDirTimer) clearTimeout(outputDirTimer)
})
</script>
