<template>
  <div class="meeting-container">
    <!-- 简化的会议界面 -->
    <header class="meeting-header">
      <div class="meeting-title">
        {{ joined ? `房间: ${room}` : 'Sparkle Meeting' }}
      </div>
      <div class="meeting-actions">
        <router-link to="/settings" class="control-btn" title="设置">
          ⚙️
          <span class="tooltip">设置</span>
        </router-link>
        <button @click="showLogs = !showLogs" class="control-btn" title="日志">
          📋
          <span class="tooltip">日志</span>
        </button>
        <button @click="showInfo = !showInfo" class="control-btn" title="信息">
          ℹ️
          <span class="tooltip">会议信息</span>
        </button>
        <button @click="refreshConnection" class="control-btn" title="刷新连接" v-if="joined">
          🔄
          <span class="tooltip">刷新连接</span>
        </button>
      </div>
    </header>

    <main class="meeting-content">
      <!-- 视频区域 -->
      <section class="video-area">
        <!-- 本地视频 -->
        <div class="video-item local-video" v-if="joined && localStream">
          <video ref="localVideo" autoplay playsinline muted></video>
          <div class="video-label">{{ userName }} (我)</div>
        </div>

        <!-- 远程视频容器 -->
        <div id="remote-videos" class="remote-videos" v-if="joined">
          <!-- 远程视频将动态添加到这里 -->
        </div>

        <!-- 未加入时的界面 -->
        <div v-if="!joined" class="join-interface">
          <div class="join-panel">
            <h2>加入会议</h2>
            <div class="join-form">
              <div class="form-group">
                <label>房间号</label>
                <input v-model="room" placeholder="请输入房间号" />
              </div>
              <div class="form-group">
                <label>用户名</label>
                <input v-model="userName" placeholder="请输入用户名" />
              </div>
              <div class="form-actions">
                <button @click="join" class="btn-primary" :disabled="joining">
                  {{ joining ? '加入中...' : '加入会议' }}
                </button>
                <button @click="createRoom" class="btn-success" :disabled="joining">
                  {{ joining ? '创建中...' : '创建房间' }}
                </button>
              </div>
            </div>
          </div>
        </div>

        <!-- 状态提示 -->
        <div v-if="status" class="status-message" :class="getStatusClass()">
          {{ status }}
        </div>
      </section>

      <!-- 调试信息 -->
      <section v-if="joined" class="debug-info" style="position: fixed; top: 10px; left: 10px; background: rgba(0,0,0,0.8); color: white; padding: 10px; border-radius: 5px; font-size: 12px; z-index: 1000;">
        <div>joined: {{ joined }}</div>
        <div>localStream: {{ !!localStream }}</div>
        <div>localVideo element: {{ !!localVideo }}</div>
        <div>videoEnabled: {{ videoEnabled }}</div>
        <div>video tracks: {{ videoTrackCount }}</div>
        <div>audio tracks: {{ audioTrackCount }}</div>
      </section>
    </main>

    <!-- 控制栏 -->
    <footer class="meeting-controls" v-if="joined">
      <div class="control-group control-group-left">
        <!-- 麦克风控制 -->
        <button @click="toggleAudio" class="control-btn" :class="{ active: !audioEnabled }">
          {{ audioEnabled ? '🎤' : '🔇' }}
          <span class="tooltip">{{ audioEnabled ? '静音' : '取消静音' }}</span>
        </button>

        <!-- 摄像头控制 -->
        <button @click="toggleVideo" class="control-btn" :class="{ active: !videoEnabled }">
          {{ videoEnabled ? '📹' : '📷' }}
          <span class="tooltip">{{ videoEnabled ? '关闭视频' : '开启视频' }}</span>
        </button>

        <!-- 屏幕共享 -->
        <button @click="toggleScreenShare" class="control-btn" :class="{ active: isScreenSharing }">
          {{ isScreenSharing ? '🖥️' : '🖥️' }}
          <span class="tooltip">{{ isScreenSharing ? '停止共享' : '共享屏幕' }}</span>
        </button>

        <!-- 画质切换 -->
        <button @click="toggleQuality" class="control-btn" :class="{ active: isLowQuality }" title="切换画质">
          {{ isLowQuality ? 'SD' : 'HD' }}
          <span class="tooltip">{{ isLowQuality ? '高清' : '标清' }}</span>
        </button>
      </div>

      <div class="control-group">
        <!-- 音量控制 -->
        <div class="volume-control">
          <span>🔊</span>
          <input
            type="range"
            v-model="globalVolume"
            min="0"
            max="100"
            @input="updateGlobalVolume"
            class="volume-slider"
          />
          <span class="volume-value">{{ globalVolume }}%</span>
        </div>
      </div>

      <div class="control-group control-group-right">
        <!-- 设备切换 -->
        <div class="device-selector">
          <select v-model="selectedAudioInput" @change="switchAudioDevice" class="device-select">
            <option value="">🎤 麦克风</option>
            <option v-for="device in audioInputDevices" :key="device.deviceId" :value="device.deviceId">
              {{ device.label || `麦克风 ${device.deviceId.slice(0, 8)}` }}
            </option>
          </select>
        </div>

        <div class="device-selector">
          <select v-model="selectedVideoInput" @change="switchVideoDevice" class="device-select">
            <option value="">📹 摄像头</option>
            <option v-for="device in videoInputDevices" :key="device.deviceId" :value="device.deviceId">
              {{ device.label || `摄像头 ${device.deviceId.slice(0, 8)}` }}
            </option>
          </select>
        </div>

        <!-- 离开会议 -->
        <button @click="leave" class="control-btn danger">
          📞
          <span class="tooltip">离开会议</span>
        </button>
      </div>
    </footer>

    <!-- 会议信息面板 -->
    <aside class="meeting-info" v-if="joined && showInfo">
      <div class="info-section">
        <div class="info-title">📊 会议信息</div>
        <div class="info-item">
          <span class="info-label">房间号</span>
          <span class="info-value">{{ room }}</span>
        </div>
        <div class="info-item">
          <span class="info-label">用户名</span>
          <span class="info-value">{{ userName }} <small>({{ userId }})</small></span>
        </div>
        <div class="info-item">
          <span class="info-label">状态</span>
          <span class="info-value">{{ joined ? '已连接' : '未连接' }}</span>
        </div>
        <div class="info-item">
          <span class="info-label">连接用户</span>
          <span class="info-value">{{ connectedUsers.length }} 人</span>
        </div>
      </div>
    </aside>

    <!-- 日志面板 -->
    <aside class="log-panel" v-if="showLogs">
      <div class="log-header">
        <div class="log-title">📋 系统日志</div>
        <button @click="logs = []; addLog('info', '日志已清空')" class="log-clear">清空</button>
      </div>
      <div class="log-content" ref="logContent">
        <div
          v-for="(log, index) in logs"
          :key="index"
          class="log-entry"
          :class="`log-${log.level}`"
        >
          <span class="log-time">{{ log.timestamp }}</span>
          <span class="log-message">{{ log.message }}</span>
        </div>
        <div v-if="logs.length === 0" class="log-empty">暂无日志</div>
      </div>
    </aside>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, nextTick, onMounted, onUnmounted, onActivated, onDeactivated } from 'vue'
import '../styles/call.css'

/* ====================== 状态 ====================== */
const room = ref('')
const userName = ref('')
const userId = ref('')
const joined = ref(false)
const joining = ref(false)
const status = ref('')
const audioEnabled = ref(true)
const videoEnabled = ref(true)
const connectedUsers = ref<string[]>([])
const showInfo = ref(false)
const isScreenSharing = ref(false)
const isLowQuality = ref(false)
const globalVolume = ref(100)

// 日志系统
interface LogEntry {
  timestamp: string
  level: 'info' | 'success' | 'warning' | 'error'
  message: string
}
const logs = ref<LogEntry[]>([])
const showLogs = ref(false)

const localVideo = ref<HTMLVideoElement | null>(null)
const logContent = ref<HTMLDivElement | null>(null)

/* ====================== 配置 ====================== */
interface ServerConfig {
  host: string
  port: number
}

interface IceServerConfig {
  urls: string[]
  username?: string
  credential?: string
}

interface AppConfig {
  server: ServerConfig
  ice_servers: IceServerConfig[]
  default_volume: number
}

const config = ref<AppConfig>({
  server: {
    host: 'localhost',
    port: 9090
  },
  ice_servers: [],
  default_volume: 50
})

async function loadAppConfig() {
  try {
    const { invoke } = await import('@tauri-apps/api/core')
    const data = await invoke<AppConfig>('load_config')
    config.value = data
    addLog('info', `已加载配置: ${data.server.host}:${data.server.port}, ICE=${data.ice_servers.length}个`)
  } catch (err) {
    addLog('warning', `加载配置失败，使用默认值: ${(err as Error).message}`)
  }
}

/* ====================== 日志函数 ====================== */
function addLog(level: 'info' | 'success' | 'warning' | 'error', message: string) {
  const now = new Date()
  const timestamp = now.toLocaleTimeString('zh-CN', { hour12: false })
  logs.value.push({ timestamp, level, message })

  // 自动滚动到底部
  nextTick(() => {
    if (logContent.value) {
      logContent.value.scrollTop = logContent.value.scrollHeight
    }
  })

  // 同时输出到控制台
  const consoleMsg = `[${timestamp}] ${message}`
  switch (level) {
    case 'info': console.log(consoleMsg); break
    case 'success': console.log('%c' + consoleMsg, 'color: green'); break
    case 'warning': console.warn(consoleMsg); break
    case 'error': console.error(consoleMsg); break
  }
}

/* ====================== 设备 ====================== */
const audioInputDevices = ref<MediaDeviceInfo[]>([])
const videoInputDevices = ref<MediaDeviceInfo[]>([])
const selectedAudioInput = ref('')
const selectedVideoInput = ref('')

const videoTrackCount = computed(() => localStream?.getVideoTracks().length ?? 0)
const audioTrackCount = computed(() => localStream?.getAudioTracks().length ?? 0)

/* ====================== 媒体流 ====================== */
let localStream: MediaStream | null = null
let ws: WebSocket | null = null

/* ====================== PeerConnections ====================== */
// 统一使用一个 PC (SFU 模式：单连接双向收发)
let pc: RTCPeerConnection | null = null
// ICE 候选缓存（在 remote description 设置之前到达的候选）
const pendingIceCandidates: RTCIceCandidateInit[] = []

/* ====================== 远端媒体 ====================== */
// 音频元素映射 (userId -> audio element)
const remoteAudioElements = new Map<string, HTMLAudioElement>()

// 每个 video stream 一个 video
const remoteVideoStreams = new Map<string, MediaStream>()

/* ====================== 服务器 ====================== */
const API_BASE = computed(() => {
  // 从配置拼接 HTTP API 地址
  const { host, port } = config.value.server
  return `http://${host}:${port}`
})
const SIGNALING_PATH = '/api/ws'

const ICE_SERVERS = computed(() => {
  return config.value.ice_servers.map(s => ({
    urls: s.urls,
    username: s.username,
    credential: s.credential
  }))
})

/* ====================== 工具 ====================== */
/* ====================== PeerConnection ====================== */

// 创建 PeerConnection (统一处理发送和接收)
function createPeerConnection() {
  pc = new RTCPeerConnection({ iceServers: ICE_SERVERS.value })

  // 先添加接收 transceiver（SFU 需要）
  pc.addTransceiver('audio', { direction: 'sendrecv' })
  pc.addTransceiver('video', { direction: 'sendrecv' })
  addLog('info', '添加音视频 transceiver (sendrecv)')

  // 添加本地轨道
  if (localStream) {
    localStream.getTracks().forEach(t => {
      pc!.addTrack(t, localStream!)
      addLog('info', `添加本地轨道: ${t.kind}, id: ${t.id}`)
    })
  }

  /* ---------- ontrack（核心） ---------- */
  pc.ontrack = (e: RTCTrackEvent) => {
    const track = e.track
    // 如果 streams 为空（SFU 常见情况），手动创建一个新流
    const stream = e.streams[0] || new MediaStream([track])
    // 尝试从 stream.id 获取用户标识，或者直接使用 stream.id
    const remoteId = stream.id

    addLog('success', `收到轨道: ${track.kind}, StreamID: ${stream.id}`)

    // 🔊 音频
    if (track.kind === 'audio') {
      const audioEl = document.createElement('audio')
      audioEl.autoplay = true
      audioEl.srcObject = stream
      audioEl.volume = globalVolume.value / 100
      document.body.appendChild(audioEl)
      // 使用 stream.id 作为 key
      remoteAudioElements.set(remoteId, audioEl)
      return
    }

    // 📹 视频
    if (track.kind === 'video') {
      if (!remoteVideoStreams.has(stream.id)) {
        remoteVideoStreams.set(stream.id, stream)
        addRemoteVideo(stream.id, stream, remoteId)
      }
    }
  }

  // 监听连接状态
  pc.onconnectionstatechange = () => {
    addLog('info', `PC 状态: ${pc!.connectionState}`)
    if (pc!.connectionState === 'failed') {
      addLog('warning', '连接失败，尝试重启 ICE...')
      refreshConnection()
    }
  }

  pc.oniceconnectionstatechange = () => {
    addLog('info', `ICE 状态: ${pc!.iceConnectionState}`)
  }

  // 监听 ICE 候选
  pc.onicecandidate = (e) => {
    if (e.candidate) {
      sendSignal('ice', { candidate: e.candidate })
      addLog('info', '发送 ICE 候选')
    }
  }

  return pc
}

// 替换本地轨道
async function replaceTrack(kind: 'audio' | 'video', newTrack: MediaStreamTrack | null) {
  if (!pc) return

  const senders = pc.getSenders()
  const sender = senders.find(s => s.track?.kind === kind)

  if (!sender) {
    addLog('warning', `未找到 ${kind} sender`)
    return
  }

  if (newTrack) {
    // 替换轨道
    await sender.replaceTrack(newTrack)
    addLog('info', `替换 ${kind} 轨道`)
  } else {
    // 移除轨道
    pc.removeTrack(sender)
    addLog('info', `移除 ${kind} 轨道，开始重新协商`)

    // 创建新的 offer
    const offer = await pc.createOffer()
    await pc.setLocalDescription(offer)

    // 发送新的 offer
    sendSignal('offer', { sdp: offer.sdp })
    addLog('info', '发送重新协商的 Offer')
  }
}

// 刷新连接
async function refreshConnection() {
  if (!pc) return
  addLog('info', '正在刷新连接 (ICE Restart)...')
  // 创建带有 iceRestart 的 Offer
  const offer = await pc.createOffer({ iceRestart: true })
  await pc.setLocalDescription(offer)
  sendSignal('offer', { sdp: offer.sdp })
}

/* ====================== Video DOM ====================== */
function addRemoteVideo(id: string, stream: MediaStream, remoteUserId: string) {
  const container = document.getElementById('remote-videos')
  if (!container) return

  if (container.querySelector(`[data-stream-id="${id}"]`)) return

  const wrap = document.createElement('div')
  wrap.className = 'video-item remote-video'
  wrap.dataset.streamId = id
  wrap.dataset.userId = remoteUserId

  const video = document.createElement('video')
  video.autoplay = true
  video.playsInline = true
  video.muted = true  // 静音视频元素，音频由单独的 audio 元素处理
  video.srcObject = stream
  // 确保视频播放
  video.onloadedmetadata = () => {
    video.play().catch(e => console.error('Remote video play failed:', e))
  }

  const label = document.createElement('div')
  label.className = 'video-label'
  label.textContent = remoteUserId

  wrap.appendChild(video)
  wrap.appendChild(label)
  container.appendChild(wrap)
}

/* ====================== 信令 ====================== */
function sendSignal(type: string, data: any) {
  const message = JSON.stringify({ type, data })
  addLog('info', `发送信令: ${type}, 大小: ${message.length} 字节`)
  ws?.send(message)
}

async function handleSignal(msg: any) {
  switch (msg.type) {
    case 'welcome':
      userId.value = msg.from_user
      addLog('success', `加入成功，我的ID: ${userId.value}`)
      break

    case 'answer':
      // 本地 offer 的 answer
      if (pc) {
        await pc.setRemoteDescription({
          type: 'answer',
          sdp: msg.data.sdp
        })
        addLog('success', '设置远程描述 (Answer)')
        status.value = '已连接到服务器'
        // 添加缓存的 ICE 候选
        while (pendingIceCandidates.length > 0) {
          const candidate = pendingIceCandidates.shift()!
          try {
            await pc.addIceCandidate(candidate)
            addLog('info', '添加缓存的 ICE 候选')
          } catch (e) {
            addLog('error', `添加缓存 ICE 候选失败: ${(e as Error).message}`)
          }
        }
      }
      break

    case 'offer':
      // SFU 发来的 Offer (通常是新用户加入时的重新协商)
      addLog('info', `收到 Offer (重新协商)`)
      if (!pc) return

      await pc.setRemoteDescription({
        type: 'offer',
        sdp: msg.data.sdp
      })
      const answer = await pc.createAnswer()
      await pc.setLocalDescription(answer)
      sendSignal('answer', { sdp: answer.sdp })
      addLog('info', `发送 Answer`)
      // 添加缓存的 ICE 候选
      while (pendingIceCandidates.length > 0) {
        const candidate = pendingIceCandidates.shift()!
        try {
          await pc.addIceCandidate(candidate)
          addLog('info', '添加缓存的 ICE 候选')
        } catch (e) {
          addLog('error', `添加缓存 ICE 候选失败: ${(e as Error).message}`)
        }
      }
      break

    case 'ice':
    case 'candidate':
      // ICE 候选 (支持两种类型名)
      if (pc) {
        // 如果远程描述还未设置，先缓存
        if (!pc.remoteDescription) {
          pendingIceCandidates.push(msg.data)
          addLog('info', '缓存 ICE 候选（等待远程描述）')
          return
        }
        try {
          const candidate = new RTCIceCandidate(msg.data);
          await pc.addIceCandidate(candidate);
        } catch (e) {
          console.error('Error adding ICE candidate:', e);
          addLog('error', `添加 ICE 候选失败: ${(e as Error).message}`);
          return;
        }


        addLog('info', `添加 ICE 候选`)
      }
      break

    case 'user_joined':
      // 新用户加入，需要创建与其的连接
      if (msg.data.user_id !== userId.value) {
        connectedUsers.value.push(msg.data.user_id)
        status.value = `${msg.data.user_id} 加入了会议`
        addLog('success', `${msg.data.user_id} 加入了会议`)
      }
      break

    case 'user_left':
      // 用户离开
      const leftUserId = msg.data.user_id
      
      // 清理视频 DOM 和流引用
      // 注意：由于单 PC 模式下我们可能没有完美的 userId 映射，这里尝试通过 data-user-id 清理
      const videoElements = document.querySelectorAll(`[data-user-id="${leftUserId}"]`)
      videoElements.forEach(el => {
        const streamId = (el as HTMLElement).dataset.streamId
        if (streamId) remoteVideoStreams.delete(streamId)
        el.remove()
      })

      connectedUsers.value = connectedUsers.value.filter(id => id !== leftUserId)
      status.value = `${leftUserId} 离开了会议`
      addLog('warning', `${leftUserId} 离开了会议`)
      break

    default:
      addLog('warning', `未知信令类型: ${msg.type}`)
  }
}

/* ====================== 加入会议 ====================== */
async function join() {
  joining.value = true
  status.value = '正在加入房间...'
  addLog('info', `开始加入房间: ${room.value}`)

  try {
    addLog('info', '请求本地媒体流...')
    localStream = await navigator.mediaDevices.getUserMedia({
      audio: true,
      video: true
    })
    addLog('success', `获取媒体流成功: ${localStream.getAudioTracks().length} 音频轨道, ${localStream.getVideoTracks().length} 视频轨道`)

    joined.value = true
    await nextTick()

    if (localVideo.value) {
      localVideo.value.srcObject = localStream
    }

    // 创建 PC
    createPeerConnection()
    addLog('info', '创建 PeerConnection')

    const wsUrl =
      `ws://${API_BASE.value.replace(/^http?:\/\//, '')}` +
      `${SIGNALING_PATH}?room_id=${room.value}&name=${userName.value}`

    addLog('info', `连接 WebSocket: ${wsUrl}`)
    ws = new WebSocket(wsUrl)

    ws.onopen = async () => {
      addLog('success', 'WebSocket 连接成功')
      // 创建 offer 并发送
      const offer = await pc!.createOffer({
        offerToReceiveAudio: true,
        offerToReceiveVideo: true
      })
      await pc!.setLocalDescription(offer)
      sendSignal('offer', { sdp: offer.sdp })
      addLog('info', '发送 Offer SDP')

      // 设置超时检测
      setTimeout(() => {
        if (pc?.remoteDescription === null) {
          addLog('warning', '未收到服务器响应，请检查服务器日志')
          status.value = '连接超时'
        }
      }, 10000) // 10秒超时

      // 重置加入状态
      joining.value = false
      status.value = '已连接'

    }

    ws.onmessage = e => {
      try {
        const msg = JSON.parse(e.data)
        addLog('info', `收到信令消息: ${msg.type}, 数据: ${msg.data ? JSON.stringify(msg.data).substring(0, 100) : '无'}...`)
        handleSignal(msg)
      } catch (err) {
        addLog('error', `解析信令消息失败: ${err}, 原始数据: ${e.data}`)
      }
    }

    ws.onerror = () => {
      addLog('error', `WebSocket 错误`)
      status.value = '连接错误'
    }

    ws.onclose = () => {
      addLog('warning', 'WebSocket 连接关闭')
      status.value = '连接已断开'

    }

    // 获取设备列表
    await loadDevices()
    addLog('info', `加载设备: ${audioInputDevices.value.length} 麦克风, ${videoInputDevices.value.length} 摄像头`)

  } catch (err) {
    addLog('error', `加入失败: ${(err as Error).message}`)
    console.error('Failed to join:', err)
    status.value = '加入失败: ' + (err as Error).message
    joining.value = false
  }
}

/* ====================== 创建房间 ====================== */
async function createRoom() {
  joining.value = true
  status.value = '正在创建房间...'
  addLog('info', '开始创建房间...')

  try {
    // 调用后端 API 创建房间
    addLog('info', `请求 ${API_BASE.value}/api/create`)
    const response = await fetch(`${API_BASE.value}/api/create`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json'
      },
      body: JSON.stringify({
        name: `${userName.value}的房间`,
        limits: 10
      })
    })

    if (!response.ok) {
      throw new Error(`HTTP ${response.status}: ${response.statusText}`)
    }

    const data = await response.json() as { room_id: string; master_id: string }

    room.value = data.room_id
    addLog('success', `房间创建成功: room_id=${data.room_id}, master_id=${data.master_id}`)

    // 使用创建的房间号加入
    await join()
  } catch (err) {
    addLog('error', `创建房间失败: ${(err as Error).message}`)
    console.error('Failed to create room:', err)
    status.value = '创建房间失败: ' + (err as Error).message
    joining.value = false
  }
}

/* ====================== 设备管理 ====================== */
async function loadDevices() {
  try {
    const devices = await navigator.mediaDevices.enumerateDevices()
    audioInputDevices.value = devices.filter(d => d.kind === 'audioinput')
    videoInputDevices.value = devices.filter(d => d.kind === 'videoinput')

    // 默认选择第一个设备
    if (audioInputDevices.value.length > 0 && !selectedAudioInput.value) {
      selectedAudioInput.value = audioInputDevices.value[0].deviceId
    }
    if (videoInputDevices.value.length > 0 && !selectedVideoInput.value) {
      selectedVideoInput.value = videoInputDevices.value[0].deviceId
    }
  } catch (err) {
    console.error('Failed to enumerate devices:', err)
  }
}

async function switchAudioDevice() {
  if (!selectedAudioInput.value) return

  try {
    addLog('info', '切换麦克风设备...')
    const newStream = await navigator.mediaDevices.getUserMedia({
      audio: { deviceId: { exact: selectedAudioInput.value } },
      video: false
    })

    const newAudioTrack = newStream.getAudioTracks()[0]
    if (newAudioTrack && localStream) {
      // 替换本地流中的轨道
      const oldTrack = localStream.getAudioTracks()[0]
      if (oldTrack) {
        localStream.removeTrack(oldTrack)
        oldTrack.stop()
      }
      localStream.addTrack(newAudioTrack)

      // 替换 PC 中的轨道
      await replaceTrack('audio', newAudioTrack)

      // 更新本地视频元素的流
      if (localVideo.value) {
        localVideo.value.srcObject = new MediaStream(localStream.getTracks())
      }
      addLog('success', '麦克风切换成功')
    }
  } catch (err) {
    addLog('error', `麦克风切换失败: ${(err as Error).message}`)
  }
}

async function switchVideoDevice() {
  if (!selectedVideoInput.value) return

  try {
    addLog('info', '切换摄像头设备...')
    const newStream = await navigator.mediaDevices.getUserMedia({
      audio: false,
      video: { deviceId: { exact: selectedVideoInput.value } }
    })

    const newVideoTrack = newStream.getVideoTracks()[0]
    if (newVideoTrack && localStream) {
      // 替换本地流中的轨道
      const oldTrack = localStream.getVideoTracks()[0]
      if (oldTrack) {
        localStream.removeTrack(oldTrack)
        oldTrack.stop()
      }
      localStream.addTrack(newVideoTrack)

      // 替换 PC 中的轨道
      await replaceTrack('video', newVideoTrack)

      // 更新本地视频元素的流
      if (localVideo.value) {
        localVideo.value.srcObject = new MediaStream(localStream.getTracks())
      }
      addLog('success', '摄像头切换成功')
    }
  } catch (err) {
    addLog('error', `摄像头切换失败: ${(err as Error).message}`)
  }
}

/* ====================== 音视频控制 ====================== */
async function toggleAudio() {
  audioEnabled.value = !audioEnabled.value
  const track = localStream?.getAudioTracks()[0]
  if (track) {
    track.enabled = audioEnabled.value
    addLog('info', audioEnabled.value ? '麦克风已开启' : '麦克风已静音')
  }
}

async function toggleVideo() {
  videoEnabled.value = !videoEnabled.value
  const track = localStream?.getVideoTracks()[0]
  if (track) {
    track.enabled = videoEnabled.value
    addLog('info', videoEnabled.value ? '摄像头已开启' : '摄像头已关闭')
  }
}

async function toggleScreenShare() {
  if (isScreenSharing.value) {
    // 停止屏幕共享
    addLog('info', '停止屏幕共享')
    const screenTrack = localStream?.getTracks().find(t => t.label.includes('screen'))
    if (screenTrack) {
      screenTrack.stop()
      localStream?.removeTrack(screenTrack)
    }
    isScreenSharing.value = false

    // 恢复摄像头
    try {
      const cameraStream = await navigator.mediaDevices.getUserMedia({ video: true })
      const cameraTrack = cameraStream.getVideoTracks()[0]
      if (cameraTrack && localStream && pc) {
        localStream.addTrack(cameraTrack)
        await replaceTrack('video', cameraTrack)
        if (localVideo.value) {
          localVideo.value.srcObject = new MediaStream(localStream.getTracks())
        }
        addLog('success', '已恢复摄像头')
      }
    } catch (err) {
      addLog('error', `恢复摄像头失败: ${(err as Error).message}`)
    }
  } else {
    // 开始屏幕共享
    addLog('info', '开始屏幕共享')
    try {
      const screenStream = await navigator.mediaDevices.getDisplayMedia({
        video: true,
        audio: true
      } as MediaStreamConstraints)

      const screenTrack = screenStream.getVideoTracks()[0]
      if (screenTrack && localStream && pc) {
        // 移除当前视频轨道
        const currentVideoTrack = localStream.getVideoTracks()[0]
        if (currentVideoTrack) {
          localStream.removeTrack(currentVideoTrack)
        }

        // 添加屏幕共享轨道
        localStream.addTrack(screenTrack)
        await replaceTrack('video', screenTrack)

        if (localVideo.value) {
          localVideo.value.srcObject = new MediaStream(localStream.getTracks())
        }

        // 监听用户停止共享
        screenTrack.onended = () => {
          addLog('warning', '用户停止了屏幕共享')
          toggleScreenShare()
        }

        isScreenSharing.value = true
        addLog('success', '屏幕共享已开启')
      }
    } catch (err) {
      addLog('error', `启动屏幕共享失败: ${(err as Error).message}`)
    }
  }
}

/* ====================== 画质切换 ====================== */
async function toggleQuality() {
  isLowQuality.value = !isLowQuality.value
  const low = isLowQuality.value

  if (!localStream) return
  const videoTrack = localStream.getVideoTracks()[0]
  if (!videoTrack) return

  const constraints = low
    ? { width: 480, height: 360, frameRate: 15 }
    : { width: 1280, height: 720, frameRate: 30 }

  try {
    await videoTrack.applyConstraints(constraints)
    addLog('info', `视频质量已切换为: ${low ? '低 (480p)' : '高 (720p)'}`)

    // 触发重新协商，让 SFU 知道新的编码参数
    if (pc) {
      const offer = await pc.createOffer()
      await pc.setLocalDescription(offer)
      sendSignal('offer', { sdp: offer.sdp })
      addLog('info', '发送重新协商的 Offer（画质切换）')
    }
  } catch (err) {
    addLog('error', `切换视频质量失败: ${(err as Error).message}`)
  }
}

/* ====================== 音量控制 ====================== */
function updateGlobalVolume() {
  remoteAudioElements.forEach(el => {
    el.volume = globalVolume.value / 100
  })
}

/* ====================== 状态类 ====================== */
function getStatusClass() {
  if (!status.value) return ''
  if (status.value.includes('成功') || status.value.includes('加入')) return 'success'
  if (status.value.includes('失败') || status.value.includes('错误')) return 'error'
  if (status.value.includes('离开')) return 'warning'
  return 'info'
}

/* ====================== 离开 ====================== */
function leave() {
  addLog('warning', '离开会议')

  // 清理缓存的 ICE 候选
  pendingIceCandidates.length = 0

  // 关闭 PC
  pc?.close()
  pc = null

  // 关闭 WebSocket
  ws?.close()
  ws = null

  // 停止本地流
  localStream?.getTracks().forEach(t => t.stop())
  localStream = null

  // 清理远程视频
  const videoCount = remoteVideoStreams.size
  remoteVideoStreams.clear()
  const remoteContainer = document.getElementById('remote-videos')
  if (remoteContainer) {
    remoteContainer.innerHTML = ''
  }
  if (videoCount > 0) {
    addLog('info', `清理 ${videoCount} 个远程视频流`)
  }

  // 清理音频
  remoteAudioElements.forEach(el => {
    el.pause()
    el.remove()
  })
  remoteAudioElements.clear()

  // 重置状态
  joined.value = false
  connectedUsers.value = []
  status.value = ''

  addLog('info', '资源已清理')
}

/* ====================== 生命周期 ====================== */
onMounted(async () => {
  addLog('info', '组件已加载')
  await loadAppConfig()
  addLog('info', `服务器地址: ${API_BASE.value}`)
})

onActivated(async () => {
  addLog('info', '返回会议页面')
  // 重新加载配置
  await loadAppConfig()
})

onDeactivated(() => {
  addLog('info', '离开会议页面')
})

onUnmounted(() => {
  addLog('info', '组件销毁')
  leave()
})
</script>