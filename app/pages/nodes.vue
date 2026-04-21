<template>
  <div class="nodes-page">
    <section class="page-hero">
      <div class="page-hero-copy">
        <UBadge label="Nodes" color="neutral" variant="subtle" class="page-eyebrow" />
        <h1>渲染节点</h1>
        <p class="page-note">当前仅启用本机节点。后续会在此页扩展局域网节点发现与调度状态，还没做 :v</p>
      </div>
    </section>
    <NodeList />
  </div>
</template>
