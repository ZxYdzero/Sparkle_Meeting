<script setup lang="ts">
import './styles/global.css'
</script>

<template>
  <main class="container">
    <router-view v-slot="{ Component }">
      <keep-alive>
        <component :is="Component" />
      </keep-alive>
    </router-view>
  </main>
</template>
