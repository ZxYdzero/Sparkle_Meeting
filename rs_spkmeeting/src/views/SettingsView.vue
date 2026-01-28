<template>
  <div class="settings-container">
    <header class="settings-header">
      <h1>⚙️ 设置</h1>
      <router-link to="/" class="back-btn">返回会议</router-link>
    </header>

    <main class="settings-content">
      <section class="settings-section">
        <h2>🌐 服务器配置</h2>

        <div class="form-group">
          <label>服务器地址 (IP 或域名)</label>
          <input v-model="config.server.host" placeholder="localhost" />
        </div>

        <div class="form-group">
          <label>端口</label>
          <input v-model="config.server.port" type="number" placeholder="9090" />
        </div>

        <div class="form-group">
          <label>ICE 服务器 (STUN/TURN)</label>
          <div class="ice-servers-list">
            <div v-for="(server, index) in config.ice_servers" :key="index" class="ice-server-item">
              <div class="ice-server-header">
                <span>服务器 {{ index + 1 }}</span>
                <button @click="removeIceServer(index)" class="btn-danger" :disabled="config.ice_servers.length === 0">
                  删除
                </button>
              </div>
              <div class="form-group">
                <label>URL (支持多个，用逗号分隔)</label>
                <input v-model="server.urls_input" placeholder="stun:stun.l.google.com:19302" />
              </div>
              <div class="form-group">
                <label>用户名 (可选，TURN 需要)</label>
                <input v-model="server.username" placeholder="username" />
              </div>
              <div class="form-group">
                <label>密码 (可选，TURN 需要)</label>
                <input v-model="server.credential" type="password" placeholder="password" />
              </div>
            </div>
          </div>
          <button @click="addIceServer" class="btn-secondary">+ 添加 ICE 服务器</button>
        </div>
      </section>

      <section class="settings-section">
        <h2>🔊 音频设置</h2>
        <div class="form-group">
          <label>默认音量: {{ config.default_volume }}%</label>
          <input type="range" v-model="config.default_volume" min="0" max="100" />
        </div>
      </section>

      <section class="settings-actions">
        <button @click="saveConfig" class="btn-primary" :disabled="saving">
          {{ saving ? '保存中...' : '保存配置' }}
        </button>
        <button @click="resetConfig" class="btn-danger" :disabled="resetting">
          {{ resetting ? '重置中...' : '恢复默认' }}
        </button>
      </section>

      <div v-if="message" class="message" :class="messageType">
        {{ message }}
      </div>
    </main>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import '../styles/settings.css'

interface ServerConfig {
  host: string
  port: number
}

interface IceServer {
  urls: string[]
  username?: string
  credential?: string
  urls_input?: string  // 用于输入框显示
}

interface AppConfiguration {
  server: ServerConfig
  ice_servers: IceServer[]
  default_audio_input?: string
  default_audio_output?: string
  default_volume: number
}

const config = ref<AppConfiguration>({
  server: {
    host: 'localhost',
    port: 9090
  },
  ice_servers: [],
  default_volume: 50
})

const saving = ref(false)
const resetting = ref(false)
const message = ref('')
const messageType = ref<'success' | 'error'>('success')

async function loadConfig() {
  try {
    const data = await invoke<AppConfiguration>('load_config')
    // 转换 urls_input 用于显示
    data.ice_servers.forEach(server => {
      server.urls_input = server.urls.join(', ')
    })
    config.value = data
  } catch (err) {
    showMessage(`加载配置失败: ${(err as Error).message}`, 'error')
  }
}

async function saveConfig() {
  saving.value = true
  try {
    // 准备保存的数据
    const saveData = { ...config.value }
    // 将 urls_input 转换为 urls 数组
    saveData.ice_servers = saveData.ice_servers
      .filter(server => server.urls_input && server.urls_input.trim())
      .map(server => ({
        urls: server.urls_input!.split(',').map(s => s.trim()).filter(s => s),
        username: server.username || undefined,
        credential: server.credential || undefined
      }))

    await invoke('save_config', { config: saveData })
    showMessage('配置已保存', 'success')
  } catch (err) {
    showMessage(`保存失败: ${(err as Error).message}`, 'error')
  } finally {
    saving.value = false
  }
}

async function resetConfig() {
  if (!confirm('确定要恢复默认配置吗？')) return
  resetting.value = true
  try {
    await invoke('reset_config')
    await loadConfig()
    showMessage('已恢复默认配置', 'success')
  } catch (err) {
    showMessage(`重置失败: ${(err as Error).message}`, 'error')
  } finally {
    resetting.value = false
  }
}

function addIceServer() {
  config.value.ice_servers.push({
    urls: [],
    urls_input: ''
  })
}

function removeIceServer(index: number) {
  config.value.ice_servers.splice(index, 1)
}

function showMessage(msg: string, type: 'success' | 'error') {
  message.value = msg
  messageType.value = type
  setTimeout(() => {
    message.value = ''
  }, 3000)
}

onMounted(() => {
  loadConfig()
})
</script>
