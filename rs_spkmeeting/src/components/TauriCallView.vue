<template>
  <div>
    <div id="controls">
      <label>房间: <input v-model="room" placeholder="输入房间名" /></label>
      <label>用户名: <input v-model="userId" placeholder="输入用户名" /></label>
        <button @click="join" :disabled="joined">加入房间</button>
      <button @click="createRoom" :disabled="joined">创建房间</button>
      <button @click="leave" :disabled="!joined">离开</button>
      <button @click="toggleAudio" :disabled="!joined">{{ audioEnabled ? '🔊 静音' : '🔇 取消静音' }}</button>
      <button @click="toggleVideo" :disabled="!joined">{{ videoEnabled ? '📹 关闭视频' : '📷 开启视频' }}</button>
      <button @click="toggleScreenShare" :disabled="!joined">
        {{ isScreenSharing ? '🖥️ 停止共享' : '🖥️ 共享屏幕' }}
      </button>
      <button @click="requestMediaPermissions" class="permissions-btn" title="请求媒体设备权限" :disabled="joined">
        🎤 请求权限
      </button>
      <button @click="toggleDevTools" class="debug-btn" title="打开/关闭开发者工具 (F12)">
        🔧 开发者工具
      </button>
    </div>

    <!-- 系统设备信息 -->
    <div id="system-info">
      <h3>📊 系统信息</h3>
      <div class="info-section">
        <h4>📹 摄像头/麦克风设备</h4>
        <div v-if="mediaDevices.length > 0">
          <div v-for="device in mediaDevices" :key="device.device_id" class="device-item">
            {{ device.label || `${device.kind} - ${device.device_id.slice(0, 8)}` }}
            <span class="device-kind">[{{ device.kind }}]</span>
          </div>
        </div>
        <div v-else>
          <button @click="enumerateDevices">获取设备列表</button>
        </div>
      </div>

      <div class="info-section">
        <h4>🖥️ 可用屏幕</h4>
        <div v-if="screens.length > 0">
          <div v-for="screen in screens" :key="screen.id" class="screen-item">
            {{ screen.name }} ({{ screen.width }}x{{ screen.height }})
            <span v-if="screen.is_primary" class="primary-badge">主屏幕</span>
          </div>
        </div>
        <div v-else>
          <button @click="enumerateScreens">获取屏幕列表</button>
        </div>
      </div>
    </div>

    <!-- 设备选择区域 -->
    <div id="device-controls" v-if="joined">
      <label>麦克风:
        <select v-model="selectedAudioInput" @change="switchAudioDevice" :disabled="!joined">
          <option value="">默认麦克风</option>
          <option v-for="device in audioInputDevices" :key="device.deviceId" :value="device.deviceId">
            {{ device.label || `麦克风 ${device.deviceId.slice(0, 8)}` }}
          </option>
        </select>
      </label>
      <label>扬声器:
        <select v-model="selectedAudioOutput" @change="switchAudioOutputDevice" :disabled="!joined">
          <option value="">默认扬声器</option>
          <option v-for="device in audioOutputDevices" :key="device.deviceId" :value="device.deviceId">
            {{ device.label || `扬声器 ${device.deviceId.slice(0, 8)}` }}
          </option>
        </select>
      </label>
    </div>

    <div class="video-container">
      <h3>🎥 本地视频</h3>
      <video ref="localVideo" autoplay playsinline muted></video>
      <div class="video-source-toggle">
        <button @click="setVideoSource('camera')" :class="{ active: currentVideoSource === 'camera' }" :disabled="!joined">
          📹 摄像头
        </button>
        <button @click="setVideoSource('screen')" :class="{ active: currentVideoSource === 'screen' }" :disabled="!joined">
          🖥️ 屏幕共享
        </button>
        <button @click="setVideoSource('both')" :class="{ active: currentVideoSource === 'both' }" :disabled="!joined">
          📹+🖥️ 同时
        </button>
      </div>
    </div>

    <div class="video-container">
      <h3>👥 远程视频</h3>
      <div id="remote-videos">
        <!-- 远程视频将动态添加到这里 -->
      </div>
    </div>

    <p v-if="status">📊 状态: {{ status }}</p>
    <p v-if="screenCaptureStatus">🖥️ 屏幕共享: {{ screenCaptureStatus }}</p>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';

// ---------- 可配置项 ----------
let SIGNALING_WS = 'ws://127.0.0.1:8081/ws';
const ROOMS_API = 'http://127.0.0.1:8081/rooms';
const ICE_SERVERS = [{ urls: 'stun:stun.l.google.com:19302' }];

// ---------- 本地状态 ----------
const room = ref('');
const userId = ref('');
const joined = ref(false);
const status = ref('');
const screenCaptureStatus = ref('');
const audioEnabled = ref(true);
const videoEnabled = ref(true);
const isScreenSharing = ref(false);

const localVideo = ref<HTMLVideoElement | null>(null);

// 设备相关状态 (Tauri 管理)
const mediaDevices = ref<Array<any>>([]);
const audioInputDevices = ref<Array<any>>([]);
const audioOutputDevices = ref<Array<any>>([]);
const selectedAudioInput = ref('');
const selectedAudioOutput = ref('');

const screens = ref<Array<any>>([]);
const currentScreenCapture = ref<string>('');
const currentVideoSource = ref<'camera' | 'screen' | 'both'>('camera');

let localStream: MediaStream | null = null;
let screenStream: MediaStream | null = null;
let pc: RTCPeerConnection | null = null;
let ws: WebSocket | null = null;

// 存储多个对端的连接
const peerConnections = ref<Map<string, RTCPeerConnection>>(new Map());

// 远程用户流管理 - 响应式状态
const remoteStreams = ref<Map<string, MediaStream[]>>(new Map());
const activeRemoteUsers = ref<Set<string>>(new Set());

// ---------- 简单工具: 发送信令 ----------
function sendSignal(msg: any) {
  if (!ws || ws.readyState !== WebSocket.OPEN) {
    console.warn('❌ WebSocket 未连接，无法发送消息:', msg);
    status.value = '连接已断开，请重新加入房间';
    return;
  }

  try {
    const messageStr = JSON.stringify(msg);
    console.log('📤 发送信令消息:', msg);
    ws.send(messageStr);
  } catch (error) {
    console.error('❌ 发送消息失败:', error, '消息内容:', msg);
    status.value = '发送消息失败';
  }
}

// ---------- Tauri 设备管理 ----------
async function getServerConfig() {
  try {
    const config = await invoke('get_server_config') as { websocket_url: string; api_url: string };
    console.log('✅ Server config:', config);
    SIGNALING_WS = config.websocket_url;
    return config;
  } catch (error) {
    console.error('❌ Failed to get server config:', error);
    return null;
  }
}

async function enumerateDevices() {
  try {
    console.log('🔍 正在获取媒体设备...');
    // 使用纯前端WebRTC API获取设备
    const devices = await navigator.mediaDevices.enumerateDevices();
    mediaDevices.value = devices;

    // 分类设备
    audioInputDevices.value = mediaDevices.value.filter(d => d.kind === 'audioinput');
    audioOutputDevices.value = mediaDevices.value.filter(d => d.kind === 'audiooutput');

    console.log('✅ 获取到媒体设备:', mediaDevices.value.length, '个');
  } catch (error) {
    console.error('❌ 获取设备失败:', error);
    status.value = `获取设备失败: ${error}`;
  }
}

async function enumerateScreens() {
  try {
    console.log('🖥️ 正在获取屏幕信息...');
    const available_screens = await invoke('enumerate_screens');
    screens.value = available_screens as Array<any>;
    console.log('✅ 获取到屏幕:', screens.value.length, '个');
  } catch (error) {
    console.error('❌ 获取屏幕失败:', error);
    status.value = `获取屏幕失败: ${error}`;
  }
}

async function startScreenCapture() {
  try {
    console.log('🖥️ 开始屏幕共享...');

    // 使用浏览器的原生屏幕共享 API
    const displayMediaOptions = {
      video: true,
      audio: true
    };

    screenStream = await navigator.mediaDevices.getDisplayMedia(displayMediaOptions);

    // 监听屏幕共享结束事件
    screenStream.getVideoTracks()[0].addEventListener('ended', () => {
      console.log('🖥️ 用户停止了屏幕共享');
      stopScreenCapture();
    });

    isScreenSharing.value = true;
    screenCaptureStatus.value = '正在共享屏幕';

    // 将屏幕流添加到所有现有的 PeerConnection
    updatePeerConnectionsWithScreenShare();

    console.log('✅ 屏幕共享开始成功');

    // 同时调用 Tauri 的屏幕枚举（为了设备列表显示）
    try {
      await enumerateScreens();
    } catch (e) {
      console.warn('⚠️ 获取屏幕列表失败:', e);
    }

  } catch (error) {
    console.error('❌ 屏幕共享失败:', error);

    // 更详细的错误处理
    let errorMessage = '屏幕共享失败';
    if (error instanceof Error) {
      if (error.name === 'NotAllowedError') {
        errorMessage = '用户取消了屏幕共享';
      } else if (error.name === 'NotFoundError') {
        errorMessage = '未找到可用的屏幕或窗口';
      } else if (error.name === 'NotReadableError') {
        errorMessage = '屏幕被其他应用占用';
      } else if (error.name === 'TypeError') {
        errorMessage = '屏幕共享不支持';
      }
      errorMessage += ': ' + error.message;
    }

    screenCaptureStatus.value = errorMessage;
  }
}

async function stopScreenCapture() {
  try {
    // 停止浏览器屏幕共享
    if (screenStream) {
      screenStream.getTracks().forEach(track => {
        track.stop();
      });
      screenStream = null;
    }

    // 从所有 PeerConnection 中移除屏幕流
    removeScreenShareFromPeerConnections();

    // 清理 Tauri 相关状态（如果有的话）
    if (currentScreenCapture.value) {
      try {
        await invoke('stop_screen_capture', { streamId: currentScreenCapture.value });
      } catch (e) {
        console.warn('⚠️ 停止 Tauri 屏幕捕获失败（可能未使用）:', e);
      }
      currentScreenCapture.value = '';
    }

    isScreenSharing.value = false;
    screenCaptureStatus.value = '屏幕共享已停止';
    console.log('✅ 屏幕共享已停止');
  } catch (error) {
    console.error('❌ 停止屏幕共享失败:', error);
    screenCaptureStatus.value = `停止失败: ${error}`;
  }
}

// ---------- WebRTC 和房间管理 (基于 CallView.vue) ----------
async function createRoom() {
  if (!room.value.trim()) {
    room.value = generateRoomName();
  }
  if (!userId.value.trim()) {
    userId.value = generateUserName();
  }
  await join();
}

async function join() {
  if (!room.value.trim()) {
    status.value = '请输入房间名';
    return;
  }
  if (!userId.value.trim()) {
    userId.value = generateUserName();
  }

  // 重置状态
  joined.value = false;
  status.value = '正在加入房间...';

  console.log('🚀 开始加入房间流程 - 房间:', room.value, '用户:', userId.value);

  try {
    // 先建立 WebSocket 连接
    console.log('🔌 正在连接 WebSocket 服务器...');
    ws = new WebSocket(SIGNALING_WS);

    ws.onopen = () => {
      console.log('🔌 WebSocket 连接已建立，正在获取媒体设备...');
      status.value = '正在获取媒体设备...';

      // 获取媒体流 - 添加更好的错误处理和权限检查
      requestUserMediaWithFallback()
        .then(stream => {
          localStream = stream;
          if (localVideo.value) {
            localVideo.value.srcObject = stream;
          } else {
            console.error('❌ localVideo 元素未找到');
          }

          console.log('✅ 媒体设备获取成功');
          console.log('📤 发送加入房间请求:', room.value, userId.value);
          status.value = '正在发送加入请求...';

          // 发送加入房间请求
          sendSignal({ type: 'Join', room: room.value, user: userId.value });

          // 设置超时和重试机制
          setTimeout(() => {
            if (!joined.value) {
              console.warn('⚠️ 加入房间超时，检查连接状态...');
              if (ws && ws.readyState === WebSocket.OPEN) {
                console.log('🔄 WebSocket 连接正常，可能是服务器响应延迟');
                status.value = '服务器响应较慢，请稍候...';
                // 延长等待时间
                setTimeout(() => {
                  if (!joined.value) {
                    status.value = '加入房间超时，请重试';
                  }
                }, 5000);
              } else {
                console.log('❌ WebSocket 连接异常');
                status.value = '连接失败，请检查网络';
              }
            }
          }, 5000);
        })
        .catch(e => {
          console.error('❌ 获取媒体设备失败:', e);
          handleMediaError(e);
        });
    };

    ws.onmessage = (ev) => {
      console.log('📨 收到 WebSocket 消息:', ev.data);
      handleSignalRaw(ev.data);
    };

    ws.onclose = (event) => {
      console.log('🔌 WebSocket 连接关闭:', event.code, event.reason);
      status.value = '连接已断开';
      joined.value = false;
    };

    ws.onerror = (error) => {
      console.error('❌ WebSocket 错误:', error);
      status.value = 'WebSocket连接错误';
    };

  } catch (e) {
    console.error('❌ 连接失败:', e);
    status.value = '连接失败: ' + e;
  }
}

// 处理收到的信令消息
async function handleSignalRaw(text: string) {
  let msg: any;
  try {
    msg = JSON.parse(text);
  } catch (e) {
    console.warn('❌ 无效的 JSON 消息:', text, '错误:', e);
    return;
  }

  console.log('📨 收到信令消息:', msg);
  console.log('📊 当前状态 - joined:', joined.value, 'room:', room.value, 'user:', userId.value);

  if (msg.type === 'join') {
    const currentUserId = (userId.value || '').trim().toLowerCase();
    const messageUserId = (msg.user || '').trim().toLowerCase();

    console.log('🔍 比较用户ID - 当前:', currentUserId, '消息:', messageUserId, '是否匹配:', currentUserId === messageUserId);

    if (currentUserId === messageUserId) {
      // 自己成功加入房间
      status.value = `已加入房间 ${room.value}`;
      joined.value = true;
      console.log('✅ 成功加入房间:', room.value);
      monitorConnectionState(); // 监控连接状态

      // 检查房间是否有其他人，如果有则主动建立连接
      setTimeout(async () => {
        try {
          const members = await getRoomMembers();
          console.log('🔍 查询房间成员结果:', members);

          if (members && members.length > 1) {
            // 与所有其他用户建立连接
            for (const otherUser of members) {
              if (otherUser !== userId.value) {
                console.log('🤝 检测到房间有其他用户，主动建立连接:', otherUser);

                // 检查是否已经建立了连接
                if (!peerConnections.value.has(otherUser)) {
                  const peerPc = createPeerConnectionForUser(otherUser);
                  console.log('🔗 PeerConnection 已创建，当前状态:', peerPc.connectionState);

                  const offer = await peerPc.createOffer();
                  console.log('📤 Offer 创建成功，类型:', offer.type);
                  await peerPc.setLocalDescription(offer);
                  console.log('✅ 本地描述设置成功');

                  sendSignal({ type: 'Offer', to: otherUser, from: userId.value, sdp: offer.sdp });
                  console.log('📨 Offer 已发送给:', otherUser);
                } else {
                  console.log('🔄 与用户', otherUser, '的连接已存在');
                }
              }
            }
          } else {
            console.log('🏠 房间中没有其他用户，等待用户加入...');
          }
        } catch (error) {
          console.error('❌ 建立连接时出错:', error);
        }
      }, 1000);
    } else {
      // 其他用户加入房间
      status.value = `${msg.user} 加入了房间`;
      console.log('👤 其他用户加入:', msg.user);

      // 添加到活跃用户列表
      activeRemoteUsers.value.add(msg.user);

      // 为新用户创建连接并立即发送offer
      setTimeout(async () => {
        if (!peerConnections.value.has(msg.user)) {
          console.log('🔗 为新用户创建连接:', msg.user);
          const peerPc = createPeerConnectionForUser(msg.user);

          // 等待ICE候选收集完成
          await new Promise(resolve => setTimeout(resolve, 500));

          try {
            // 主动创建并发送offer
            const offer = await peerPc.createOffer();
            await peerPc.setLocalDescription(offer);
            sendSignal({ type: 'Offer', to: msg.user, from: userId.value, sdp: offer.sdp });
            console.log('📤 已主动向新用户发送Offer:', msg.user);
          } catch (error) {
            console.error('❌ 向新用户发送Offer失败:', msg.user, error);
          }
        } else {
          console.log('🔄 与用户', msg.user, '的连接已存在');
          // 如果连接已存在，重新协商以确保媒体流更新
          await renegotiateWithUser(msg.user);
        }
      }, 1000);
    }
  } else if (msg.type === 'Offer') {
    const from = msg.from;
    const sdp = msg.sdp;
    console.log('📥 收到 Offer 来自:', from);

    let pc = peerConnections.value.get(from);
    if (!pc) {
      console.log('🔗 创建新的 PeerConnection 用于:', from);
      pc = createPeerConnectionForUser(from);
    }

    try {
      console.log('📥 开始处理 Offer，SDP 长度:', sdp ? sdp.length : 0);
      console.log('📡 PeerConnection 当前状态:', pc.connectionState);
      console.log('🧊 ICE 连接状态:', pc.iceConnectionState);

      await pc.setRemoteDescription({ type: 'offer', sdp });
      console.log('✅ 设置远程描述成功');
      console.log('📡 远程描述设置后:', pc.remoteDescription);

      const answer = await pc.createAnswer();
      console.log('📤 Answer 创建成功，类型:', answer.type);

      await pc.setLocalDescription(answer);
      console.log('✅ 本地描述设置成功');
      console.log('📡 本地描述详情:', pc.localDescription);

      console.log('📤 发送 Answer 给:', from);
      sendSignal({ type: 'Answer', to: from, from: userId.value, sdp: answer.sdp });
      console.log('📨 Answer 已发送');
    } catch (error) {
      console.error('❌ 处理 Offer 时出错:', error);
      console.error('❌ 错误详情:', {
        peerId: from,
        pcState: pc.connectionState,
        iceState: pc.iceConnectionState,
        hasLocalDesc: !!pc.localDescription,
        hasRemoteDesc: !!pc.remoteDescription
      });
    }
  } else if (msg.type === 'Answer') {
    const from = msg.from;
    const sdp = msg.sdp;
    console.log('📥 收到 Answer 来自:', from);
    const pc = peerConnections.value.get(from);
    if (pc) {
      try {
        console.log('📥 开始处理 Answer，SDP 长度:', sdp ? sdp.length : 0);
        console.log('📡 PeerConnection 当前状态:', pc.connectionState);
        console.log('📡 本地描述状态:', pc.localDescription);
        console.log('📡 远程描述状态:', pc.remoteDescription);

        await pc.setRemoteDescription({ type: 'answer', sdp });
        console.log('✅ 设置远程描述成功');
        console.log('📡 远程描述设置后:', pc.remoteDescription);
        console.log('🔗 连接状态变化:', pc.connectionState);
      } catch (error) {
        console.error('❌ 处理 Answer 时出错:', error);
        console.error('❌ 错误详情:', {
          peerId: from,
          pcState: pc.connectionState,
          iceState: pc.iceConnectionState,
          hasLocalDesc: !!pc.localDescription,
          hasRemoteDesc: !!pc.remoteDescription
        });
      }
    } else {
      console.warn('⚠️ 未找到对应的 PeerConnection:', from);
      console.warn('⚠️ 当前所有连接:', Array.from(peerConnections.value.keys()));
    }
  } else if (msg.type === 'ice') {
    console.log('🧊 收到 ICE Candidate 来自:', msg.from);
    const pc = peerConnections.value.get(msg.from);
    if (pc) {
      try {
        console.log('🧊 开始处理 ICE Candidate');
        console.log('🧊 Candidate 详情:', msg.candidate);
        console.log('📡 PeerConnection 当前状态:', pc.connectionState);
        console.log('🧊 ICE 连接状态:', pc.iceConnectionState);

        await pc.addIceCandidate(new RTCIceCandidate({
          candidate: msg.candidate,
          sdpMid: '',
          sdpMLineIndex: 0
        }));
        console.log('✅ 添加 ICE Candidate 成功');
        console.log('🧊 ICE 连接状态更新:', pc.iceConnectionState);
      } catch (err) {
        console.error('❌ 添加 ICE Candidate 失败:', err);
        console.error('❌ ICE 错误详情:', {
          peerId: msg.from,
          candidate: msg.candidate,
          pcState: pc.connectionState,
          iceState: pc.iceConnectionState,
          hasRemoteDesc: !!pc.remoteDescription
        });
      }
    } else {
      console.warn('⚠️ 未找到对应的 PeerConnection for ICE:', msg.from);
      console.warn('⚠️ 当前所有连接:', Array.from(peerConnections.value.keys()));
    }
  } else if (msg.type === 'leave') {
    const leavingUserId = msg.user;
    console.log('👤 用户离开房间:', leavingUserId);

    const currentUserId = (userId.value || '').trim().toLowerCase();
    const messageUserId = (leavingUserId || '').trim().toLowerCase();

    if (currentUserId === messageUserId) {
      console.log('👋 自己离开房间');
      status.value = '已离开房间';
      joined.value = false;
    } else {
      console.log('👋 其他用户离开房间:', leavingUserId);

      // 1. 关闭PeerConnection
      const pc = peerConnections.value.get(leavingUserId);
      if (pc) {
        try {
          // 停止所有发送器和接收器
          const senders = pc.getSenders();
          senders.forEach(sender => {
            if (sender.track) {
              sender.track.stop();
              console.log('⏹️ 停止发送轨道:', sender.track.kind);
            }
          });

          const receivers = pc.getReceivers();
          receivers.forEach(receiver => {
            if (receiver.track) {
              receiver.track.stop();
              console.log('⏹️ 停止接收轨道:', receiver.track.kind);
            }
          });

          // 关闭连接
          pc.close();
          peerConnections.value.delete(leavingUserId);
          console.log('🔗 已完全关闭与用户的连接:', leavingUserId);
        } catch (error) {
          console.error('❌ 关闭连接时出错:', leavingUserId, error);
        }
      }

      // 2. 清理DOM元素（使用简单的清理方式）
      const remoteVideoContainer = document.getElementById('remote-videos');
      if (remoteVideoContainer) {
        // 查找所有相关元素并移除
        const elements = remoteVideoContainer.querySelectorAll(`[id^="video-${leavingUserId}"], [id^="audio-${leavingUserId}"], [id^="debug-${leavingUserId}"], [data-user="${leavingUserId}"]`);
        elements.forEach(element => {
          console.log('🗑️ 移除元素:', element.id || element.tagName);

          // 如果是媒体元素，停止流
          if (element instanceof HTMLVideoElement || element instanceof HTMLAudioElement) {
            const stream = (element as HTMLVideoElement | HTMLAudioElement).srcObject as MediaStream;
            if (stream) {
              stream.getTracks().forEach(track => {
                try {
                  track.stop();
                  console.log('⏹️ 停止轨道:', track.kind);
                } catch (error) {
                  console.warn('⚠️ 停止轨道时出错:', error);
                }
              });
            }
            (element as HTMLVideoElement | HTMLAudioElement).srcObject = null;
          }

          element.remove();
        });

        // 强制重绘
        remoteVideoContainer.style.display = 'none';
        remoteVideoContainer.offsetHeight; // 触发重排
        setTimeout(() => {
          remoteVideoContainer.style.display = 'flex';
        }, 50);
      }

      // 3. 最后更新状态
      activeRemoteUsers.value.delete(leavingUserId);
      remoteStreams.value.delete(leavingUserId);

      console.log('🎥 已完成用户离开清理:', leavingUserId);
      status.value = `${leavingUserId} 离开了房间`;
    }
  } else if (msg.type === 'trigger_offer') {
    const newUser = msg.new_user || msg.target_user; // 兼容两种格式
    const action = msg.action || 'send_offer';
    const targetRoom = msg.room;
    console.log('📥 收到发送Offer触发消息，新用户:', newUser, '动作:', action, '房间:', targetRoom);

    // 验证消息类型
    if (action !== 'send_offer') {
      console.warn('⚠️ 未知的trigger_offer动作:', action);
      return;
    }

    // 验证消息是否针对当前房间
    if (targetRoom !== room.value) {
      console.warn('⚠️ 收到其他房间的触发消息，忽略:', targetRoom);
      return;
    }

    // 确保不处理对自己发的消息
    if (newUser === userId.value) {
      console.warn('⚠️ 收到对自己的trigger_offer，忽略');
      return;
    }

    // 检查是否已经有连接
    if (!peerConnections.value.has(newUser)) {
      console.log('🔗 响应触发消息，为新用户创建连接:', newUser);
      const peerPc = createPeerConnectionForUser(newUser);

      // 延迟发送Offer，确保连接已建立
      setTimeout(async () => {
        try {
          const offer = await peerPc.createOffer();
          await peerPc.setLocalDescription(offer);
          sendSignal({ type: 'Offer', to: newUser, from: userId.value, sdp: offer.sdp });
          console.log('📤 已响应触发消息，发送Offer给:', newUser);
        } catch (error) {
          console.error('❌ 响应触发消息发送Offer失败:', newUser, error);
        }
      }, 200); // 稍微增加延迟确保连接稳定
    } else {
      console.log('🔄 与用户', newUser, '的连接已存在，重新协商');
      renegotiateWithUser(newUser);
    }
  }
}


// 获取房间所有成员
async function getRoomMembers(): Promise<string[]> {
  try {
    const res = await fetch(`${ROOMS_API}/${encodeURIComponent(room.value)}/members`);
    if (!res.ok) return [];
    const j = await res.json();
    const members: string[] = j.members || [];
    return members;
  } catch (e) {
    console.warn('failed fetch room members', e);
    return [];
  }
}

// 创建与特定用户的 RTCPeerConnection
function createPeerConnectionForUser(peerId: string): RTCPeerConnection {
  console.log('🔗 创建 PeerConnection 与用户:', peerId);
  const pc = new RTCPeerConnection({ iceServers: ICE_SERVERS });

  // 添加本地流
  if (localStream) {
    localStream.getTracks().forEach(t => {
      console.log('📹 添加本地轨道:', t.kind);
      pc.addTrack(t, localStream!);
    });
  }

  // 如果正在屏幕共享，也添加屏幕流
  if (screenStream) {
    screenStream.getTracks().forEach(t => {
      console.log('🖥️ 添加屏幕轨道到新连接:', t.kind);
      pc.addTrack(t, screenStream!);
    });
  }

  pc.ontrack = (ev) => {
    console.log('📹 收到远程流来自:', peerId, ev.streams.length, '个流');
    console.log('📊 流详情:', {
      streamId: ev.streams[0]?.id,
      tracks: ev.streams[0]?.getTracks().map(t => ({
        kind: t.kind,
        enabled: t.enabled,
        muted: t.muted,
        readyState: t.readyState
      })) || []
    });

    // 添加简化的流生命周期管理
    ev.streams.forEach((stream) => {
      // 监听流轨道移除事件
      stream.addEventListener('removetrack', (event) => {
        console.log('🗑️ 轨道被移除:', peerId, event.track.kind);
        setTimeout(() => {
          if (stream.getTracks().length === 0) {
            console.log('📹 流已空，清理元素:', peerId);
            cleanupStreamElements(peerId, stream.id);
          }
        }, 50);
      });

      // 监听流结束事件
      stream.addEventListener('ended', () => {
        console.log('📹 流结束:', peerId, stream.id);
        setTimeout(() => {
          cleanupStreamElements(peerId, stream.id);
        }, 50);
      });

      // 监听轨道结束事件
      stream.getTracks().forEach(track => {
        track.addEventListener('ended', () => {
          console.log('⏹️ 轨道结束:', peerId, track.kind);
          setTimeout(() => {
            const hasActiveTracks = stream.getTracks().some(t => t.readyState === 'live');
            if (!hasActiveTracks) {
              cleanupStreamElements(peerId, stream.id);
            }
          }, 100);
        });

        // 监听轨道静音状态变化
        track.addEventListener('mute', () => {
          console.log('🔇 轨道静音:', peerId, track.kind);
        });

        track.addEventListener('unmute', () => {
          console.log('🔊 轨道取消静音:', peerId, track.kind);
        });
      });
    });

    const remoteVideoContainer = document.getElementById('remote-videos');
    if (!remoteVideoContainer) {
      console.error('❌ 未找到远程视频容器 #remote-videos');
      return;
    }

    // 检测是否在Tauri环境中
    const isTauri = typeof window !== 'undefined' && '__TAURI__' in window;
    console.log('🌍 当前环境:', isTauri ? 'Tauri' : 'Browser');

    // 检查是否是新的流（避免重复处理）
    ev.streams.forEach((stream, streamIndex) => {
      const tracks = stream.getTracks();
      const hasVideo = tracks.some(t => t.kind === 'video');
      const hasAudio = tracks.some(t => t.kind === 'audio');

      console.log(`📹 处理流 ${streamIndex}:`, {
        streamId: stream.id,
        hasVideo,
        hasAudio,
        tracksCount: tracks.length,
        trackKinds: tracks.map(t => t.kind)
      });

      // 检查是否已经存在这个流的视频元素
      const videoElementId = `video-${peerId}-${streamIndex}`;
      const existingVideoElement = document.getElementById(videoElementId) as HTMLVideoElement;

      if (existingVideoElement) {
        console.log(`🔄 视频元素已存在，更新流: ${peerId} 流 ${streamIndex}`);
        existingVideoElement.srcObject = stream;
        return; // 跳过后续处理
      }

      // 为每个视频流创建单独的视频元素
      if (hasVideo) {
        let videoElement = document.getElementById(videoElementId) as HTMLVideoElement;

        if (!videoElement) {
          console.log(`🎥 创建新的视频元素用于: ${peerId} 流 ${streamIndex}`);
          videoElement = document.createElement('video');
          videoElement.id = videoElementId;

          // Tauri特定的视频元素设置
          videoElement.autoplay = true;
          videoElement.playsInline = true;
          videoElement.muted = false; // 确保不静音远程音频
          videoElement.controls = true; // 添加控制按钮以便调试
          videoElement.style.width = '45%';
          videoElement.style.border = isTauri ? '3px solid #ff6b6b' : '2px solid #007bff';
          videoElement.style.margin = '4px';
          videoElement.style.borderRadius = '8px';
          videoElement.style.backgroundColor = '#000';
          videoElement.style.objectFit = 'contain'; // 确保视频正确显示

          // 根据流类型设置不同的标题和样式
          const isScreenShare = tracks.some(t => t.kind === 'video' && t.label && t.label.toLowerCase().includes('screen'));
          const streamType = isScreenShare ? '屏幕共享' : '摄像头';
          videoElement.title = `${isTauri ? '[Tauri] ' : ''}远程用户: ${peerId} (${streamType})`;

          // 为视频元素添加用户标识属性
          videoElement.setAttribute('data-user', peerId);
          videoElement.setAttribute('data-stream-id', stream.id);
          videoElement.setAttribute('data-stream-index', streamIndex.toString());

          if (isScreenShare) {
            videoElement.style.border = isTauri ? '3px solid #28a745' : '2px solid #28a745'; // 绿色表示屏幕共享
          }

          // 添加事件监听器来调试视频播放
          videoElement.addEventListener('loadedmetadata', () => {
            console.log(`🎥 视频元数据已加载: ${peerId} 流 ${streamIndex}`, {
              videoWidth: videoElement.videoWidth,
              videoHeight: videoElement.videoHeight,
              duration: videoElement.duration,
              currentTime: videoElement.currentTime,
              environment: isTauri ? 'Tauri' : 'Browser',
              streamType
            });
          });

          videoElement.addEventListener('loadeddata', () => {
            console.log(`🎥 视频数据已加载: ${peerId} 流 ${streamIndex}`);
            // 在Tauri环境中强制播放
            if (isTauri) {
              setTimeout(() => {
                const playPromise = videoElement.play();
                if (playPromise !== undefined) {
                  playPromise.then(() => {
                    console.log(`🎥 Tauri强制播放成功: ${peerId} 流 ${streamIndex}`);
                  }).catch(e => {
                    console.log(`🎥 Tauri强制播放被阻止: ${peerId} 流 ${streamIndex}`, e);
                    // 尝试用户交互触发播放
                    videoElement.addEventListener('click', () => {
                      videoElement.play().catch(err => {
                        console.log(`🎥 点击播放也失败: ${peerId} 流 ${streamIndex}`, err);
                      });
                    }, { once: true });
                  });
                }
              }, 100);
            }
          });

          videoElement.addEventListener('play', () => {
            console.log(`🎥 视频开始播放: ${peerId} 流 ${streamIndex}`);
          });

          videoElement.addEventListener('pause', () => {
            console.log(`🎥 视频暂停: ${peerId} 流 ${streamIndex}`);
          });

          videoElement.addEventListener('error', (e) => {
            console.error(`❌ 视频播放错误: ${peerId} 流 ${streamIndex}`, e);
            if (isTauri) {
              console.log(`💡 Tauri环境视频错误 - 尝试重新设置源: ${peerId} 流 ${streamIndex}`);
              // 在Tauri中尝试重新设置视频源
              setTimeout(() => {
                videoElement.srcObject = stream;
                videoElement.load();
                videoElement.play().catch(playErr => {
                  console.log(`🎥 重新设置源后播放失败: ${peerId} 流 ${streamIndex}`, playErr);
                });
              }, 500);
            }
          });

          // Tauri特定的事件监听器
          if (isTauri) {
            videoElement.addEventListener('canplay', () => {
              console.log(`🎥 Tauri视频可以播放: ${peerId} 流 ${streamIndex}`);
              videoElement.play().catch(e => {
                console.log(`🎥 Tauri自动播放被阻止: ${peerId} 流 ${streamIndex}`, e);
              });
            });

            videoElement.addEventListener('stalled', () => {
              console.log(`⚠️ Tauri视频加载停滞: ${peerId} 流 ${streamIndex}`);
            });

            videoElement.addEventListener('waiting', () => {
              console.log(`⏳ Tauri视频等待数据: ${peerId} 流 ${streamIndex}`);
            });

            videoElement.addEventListener('progress', () => {
              console.log(`📊 Tauri视频加载进度: ${peerId} 流 ${streamIndex}`);
            });
          }

          remoteVideoContainer.appendChild(videoElement);
          console.log(`✅ 视频元素已添加到DOM: ${peerId} 流 ${streamIndex}, 环境:`, isTauri ? 'Tauri' : 'Browser');
        } else {
          console.log(`🔄 复用现有视频元素: ${peerId} 流 ${streamIndex}`);
        }

        // 设置视频源流
        videoElement.srcObject = stream;
        console.log(`📹 设置视频源流成功: ${peerId} 流 ${streamIndex}, 流ID:`, stream.id);

        // 在Tauri环境中强制重新加载视频
        if (isTauri) {
          console.log(`🔄 Tauri环境强制重新加载视频: ${peerId} 流 ${streamIndex}`);
          videoElement.load(); // 强制重新加载视频
        }

        // 添加一个调试标签
        const debugInfoId = `debug-${peerId}-${streamIndex}`;
        const debugInfo = document.createElement('div');
        debugInfo.style.fontSize = '12px';
        debugInfo.style.color = isTauri ? '#ff6b6b' : '#666';
        debugInfo.style.marginTop = '4px';
        debugInfo.style.fontWeight = isTauri ? 'bold' : 'normal';
        const streamType = tracks.some(t => t.kind === 'video' && t.label && t.label.toLowerCase().includes('screen')) ? '屏幕共享' : '摄像头';
        debugInfo.textContent = `${isTauri ? '[Tauri] ' : ''}用户: ${peerId} (${streamType}) | 轨道: ${stream.getTracks().length} | ${new Date().toLocaleTimeString()}`;

        // 更新或添加调试信息
        let existingDebug = document.getElementById(debugInfoId);
        if (!existingDebug) {
          existingDebug = debugInfo;
          existingDebug.id = debugInfoId;
          videoElement.parentNode?.insertBefore(existingDebug, videoElement.nextSibling);
        } else {
          existingDebug.textContent = debugInfo.textContent;
        }

        // Tauri特定的延迟处理
        if (isTauri) {
          setTimeout(() => {
            console.log(`🔄 Tauri延迟检查视频状态: ${peerId} 流 ${streamIndex}`);
            if (videoElement.paused) {
              console.log(`🎥 Tauri视频仍暂停，尝试播放: ${peerId} 流 ${streamIndex}`);
              videoElement.play().catch(e => {
                console.log(`🎥 Tauri延迟播放失败: ${peerId} 流 ${streamIndex}`, e);
              });
            }
          }, 1000);
        }
      }

      // 为音频轨道创建独立的音频元素
      if (hasAudio) {
        console.log(`🔊 处理音频轨道: ${peerId} 流 ${streamIndex}`, stream.getAudioTracks().length, '个音频轨道');

        const audioElementId = `audio-${peerId}-${streamIndex}`;
        let audioElement = document.getElementById(audioElementId) as HTMLAudioElement;

        if (!audioElement) {
          audioElement = document.createElement('audio');
          audioElement.id = audioElementId;
          audioElement.autoplay = true;
          audioElement.muted = false; // 确保不静音远程音频
          audioElement.controls = true; // 显示控制器以便用户调试
          audioElement.style.width = '100%';
          audioElement.style.marginTop = '4px';
          audioElement.style.border = isTauri ? '2px solid #ff6b6b' : '1px solid #007bff';
          audioElement.style.borderRadius = '4px';
          audioElement.style.backgroundColor = '#f8f9fa';
          audioElement.title = `${isTauri ? '[Tauri] ' : ''}远程音频: ${peerId} 流 ${streamIndex}`;

          // 为音频元素添加用户标识属性
          audioElement.setAttribute('data-user', peerId);
          audioElement.setAttribute('data-stream-id', stream.id);
          audioElement.setAttribute('data-stream-index', streamIndex.toString());

          // 添加音频事件监听器
          audioElement.addEventListener('play', () => {
            console.log(`🔊 音频开始播放: ${peerId} 流 ${streamIndex}`);
            status.value = `正在播放 ${peerId} 的音频`;
          });

          audioElement.addEventListener('pause', () => {
            console.log(`🔊 音频暂停: ${peerId} 流 ${streamIndex}`);
          });

          audioElement.addEventListener('error', (e) => {
            console.error(`❌ 音频播放错误: ${peerId} 流 ${streamIndex}`, e);
            status.value = `${peerId} 音频播放失败`;
          });

          audioElement.addEventListener('loadeddata', () => {
            console.log(`🔊 音频数据已加载: ${peerId} 流 ${streamIndex}`);
          });

          audioElement.addEventListener('canplay', () => {
            console.log(`🔊 音频可以播放: ${peerId} 流 ${streamIndex}`);
          });

          remoteVideoContainer.appendChild(audioElement);
          console.log(`✅ 音频元素已添加到DOM: ${peerId} 流 ${streamIndex}`);
        }

        // 设置音频源并尝试播放
        audioElement.srcObject = stream;

        // 尝试播放音频，处理自动播放策略
        const playAudio = async () => {
          try {
            await audioElement.play();
            console.log(`🔊 音频自动播放成功: ${peerId} 流 ${streamIndex}`);
          } catch (playError) {
            console.log(`🔊 音频自动播放被阻止: ${peerId} 流 ${streamIndex}`, playError);

            // 添加用户交互事件来启动音频播放
            const startAudioOnInteraction = () => {
              audioElement.play().then(() => {
                console.log(`🔊 用户交互后音频播放成功: ${peerId} 流 ${streamIndex}`);
                status.value = `${peerId} 的音频已启动`;
              }).catch(err => {
                console.log(`🔊 用户交互后音频播放仍失败: ${peerId} 流 ${streamIndex}`, err);
              });
            };

            // 为所有视频元素添加点击事件
            const videoElements = remoteVideoContainer.querySelectorAll(`video[id^="video-${peerId}"]`);
            videoElements.forEach(videoEl => {
              videoEl.addEventListener('click', startAudioOnInteraction, { once: true });
            });
            audioElement.addEventListener('click', startAudioOnInteraction, { once: true });

            // 显示提示信息
            status.value = `点击视频或音频区域启动 ${peerId} 的音频`;

            // 在Tauri环境中显示特殊提示
            if (isTauri) {
              setTimeout(() => {
                console.log(`💡 Tauri环境音频提示: ${peerId} 流 ${streamIndex}`);
                status.value = `Tauri环境: 请点击远程视频或音频控制来播放声音`;
              }, 1000);
            }
          }
        };

        // 延迟尝试播放音频
        setTimeout(playAudio, 100);
      }
    });

    if (ev.streams.length === 0) {
      console.error('❌ 没有收到有效的流数据:', peerId);
    }
  };

  pc.onicecandidate = (ev) => {
    if (ev.candidate) {
      console.log('🧊 发送 ICE Candidate 给:', peerId);
      sendSignal({
        type: 'Ice',
        to: peerId,
        from: userId.value,
        candidate: ev.candidate.candidate
      });
    } else {
      console.log('✅ ICE 收集完成');
    }
  };

  pc.onconnectionstatechange = () => {
    const state = pc.connectionState;
    console.log('🔗 连接状态变化:', peerId, state);

    // 添加详细的状态分析
    switch (state) {
      case 'connected':
        console.log('✅ WebRTC连接已建立:', peerId);
        status.value = `已连接到 ${peerId}`;
        break;
      case 'connecting':
        console.log('🔄 正在建立连接:', peerId);
        status.value = `正在连接到 ${peerId}...`;
        break;
      case 'disconnected':
        console.log('❌ 连接已断开:', peerId);
        status.value = `与 ${peerId} 的连接已断开`;
        break;
      case 'failed':
        console.error('💥 连接失败:', peerId);
        status.value = `与 ${peerId} 的连接失败`;
        break;
      case 'closed':
        console.log('🔒 连接已关闭:', peerId);
        status.value = `与 ${peerId} 的连接已关闭`;
        break;
      default:
        console.log('❓ 未知连接状态:', peerId, state);
    }
  };

  pc.oniceconnectionstatechange = () => {
    const state = pc.iceConnectionState;
    console.log('🧊 ICE 连接状态变化:', peerId, state);

    // 添加详细的ICE状态分析
    switch (state) {
      case 'connected':
      case 'completed':
        console.log('✅ ICE连接成功:', peerId);
        break;
      case 'checking':
        console.log('🔍 ICE正在检查连接:', peerId);
        break;
      case 'disconnected':
        console.log('❌ ICE连接断开:', peerId);
        break;
      case 'failed':
        console.error('💥 ICE连接失败:', peerId);
        // 显示可能的解决方法
        console.log('💡 ICE连接失败的常见原因:');
        console.log('  1. 网络防火墙阻止了P2P连接');
        console.log('  2. NAT穿透失败');
        console.log('  3. STUN/TURN服务器不可用');
        console.log('  4. 本地网络问题');
        break;
      case 'closed':
        console.log('🔒 ICE连接已关闭:', peerId);
        break;
    }
  };

  // 添加数据通道状态监控
  pc.ondatachannel = (event) => {
    console.log('📡 数据通道已建立:', peerId, event.channel.label);
    event.channel.onopen = () => {
      console.log('📡 数据通道已打开:', peerId);
    };
    event.channel.onclose = () => {
      console.log('📡 数据通道已关闭:', peerId);
    };
    event.channel.onerror = (error) => {
      console.error('❌ 数据通道错误:', peerId, error);
    };
  };

  // 添加连接健康检查定时器
  const healthCheckInterval = setInterval(() => {
    if (pc.connectionState === 'failed' || pc.iceConnectionState === 'failed') {
      console.log('🔍 检测到连接失败，尝试重新连接:', peerId);
      clearInterval(healthCheckInterval);

      // 清理失败的连接
      pc.close();
      peerConnections.value.delete(peerId);

      // 延迟重连
      setTimeout(() => {
        console.log('🔄 重新建立连接:', peerId);
        createPeerConnectionForUser(peerId);
        renegotiateWithUser(peerId);
      }, 2000);
    } else if (pc.connectionState === 'disconnected') {
      console.log('⚠️ 连接断开，等待恢复:', peerId);
    }
  }, 5000);

  // 在连接关闭时清理定时器
  const handleConnectionClose = () => {
    clearInterval(healthCheckInterval);
    console.log('🧹 清理连接健康检查定时器:', peerId);
  };

  pc.addEventListener('connectionstatechange', () => {
    if (pc.connectionState === 'closed' || pc.connectionState === 'failed') {
      handleConnectionClose();
    }
  });

  peerConnections.value.set(peerId, pc);
  return pc;
}

// 与特定用户重新协商媒体流
async function renegotiateWithUser(peerId: string) {
  console.log('🔄 与用户重新协商:', peerId);
  const pc = peerConnections.value.get(peerId);

  if (!pc) {
    console.warn('⚠️ 未找到与用户的连接:', peerId);
    return;
  }

  try {
    // 检查连接状态
    if (pc.signalingState === 'stable') {
      const offer = await pc.createOffer();
      await pc.setLocalDescription(offer);
      sendSignal({ type: 'Offer', to: peerId, from: userId.value, sdp: offer.sdp });
      console.log('📤 重新协商Offer已发送:', peerId);
    } else {
      console.log('⏳ 连接正在协商中，稍后重试:', peerId, pc.signalingState);
      // 延迟重试
      setTimeout(() => renegotiateWithUser(peerId), 2000);
    }
  } catch (error) {
    console.error('❌ 重新协商失败:', peerId, error);
  }
}



// 清理特定流的元素
function cleanupStreamElements(peerId: string, streamId: string) {
  console.log('🧹 清理流元素:', peerId, streamId);

  const remoteVideoContainer = document.getElementById('remote-videos');
  if (!remoteVideoContainer) return;

  // 查找并移除相关的视频和音频元素
  const elements = remoteVideoContainer.querySelectorAll(`[id^="video-${peerId}"], [id^="audio-${peerId}"], [id^="debug-${peerId}"]`);

  elements.forEach(element => {
    // 检查是否是要清理的流
    const mediaElement = element as HTMLVideoElement | HTMLAudioElement;
    if (mediaElement.srcObject) {
      const stream = mediaElement.srcObject as MediaStream;
      if (stream.id === streamId || stream.getTracks().length === 0) {
        console.log('🗑️ 移除流相关元素:', element.id);

        // 停止流
        stream.getTracks().forEach(track => track.stop());
        mediaElement.srcObject = null;

        // 移除元素
        element.remove();
      }
    } else {
      // 对于调试元素，直接移除
      element.remove();
    }
  });

  // 从远程流管理中移除
  const userStreams = remoteStreams.value.get(peerId);
  if (userStreams) {
    const updatedStreams = userStreams.filter(s => s.id !== streamId);
    if (updatedStreams.length === 0) {
      remoteStreams.value.delete(peerId);
    } else {
      remoteStreams.value.set(peerId, updatedStreams);
    }
  }
}

// ---------- UI: 离开房间 ----------
function leave() {
  console.log('👋 开始离开房间流程...');

  // 通知其他用户
  sendSignal({ type: 'Leave', room: room.value, user: userId.value });

  // 停止屏幕共享
  if (isScreenSharing.value) {
    stopScreenCapture();
  }

  // 完全清理所有PeerConnection
  peerConnections.value.forEach((pc, peerId) => {
    try {
      // 停止所有发送器和接收器
      const senders = pc.getSenders();
      senders.forEach(sender => {
        if (sender.track) {
          sender.track.stop();
          console.log('⏹️ 停止发送轨道:', sender.track.kind);
        }
      });

      const receivers = pc.getReceivers();
      receivers.forEach(receiver => {
        if (receiver.track) {
          receiver.track.stop();
          console.log('⏹️ 停止接收轨道:', receiver.track.kind);
        }
      });

      pc.close();
      console.log('🔗 已关闭连接:', peerId);
    } catch (error) {
      console.error('❌ 关闭连接失败:', peerId, error);
    }
  });
  peerConnections.value.clear();

  // 清理本地流
  if (localStream) {
    localStream.getTracks().forEach(track => {
      track.stop();
      console.log('⏹️ 停止本地轨道:', track.kind);
    });
    localStream = null;
  }

  if (screenStream) {
    screenStream.getTracks().forEach(track => {
      track.stop();
      console.log('⏹️ 停止屏幕轨道:', track.kind);
    });
    screenStream = null;
  }

  // 清理所有远程媒体元素
  const remoteVideoContainer = document.getElementById('remote-videos');
  if (remoteVideoContainer) {
    const elements = remoteVideoContainer.querySelectorAll('video, audio, div');
    elements.forEach(element => {
      if (element instanceof HTMLVideoElement || element instanceof HTMLAudioElement) {
        const stream = (element as HTMLVideoElement | HTMLAudioElement).srcObject as MediaStream;
        if (stream) {
          stream.getTracks().forEach(track => track.stop());
        }
        (element as HTMLVideoElement | HTMLAudioElement).srcObject = null;
      }
      element.remove();
    });
  }

  // 清理本地视频
  if (localVideo.value) {
    localVideo.value.srcObject = null;
  }

  // 关闭WebSocket
  if (ws) {
    ws.close();
    ws = null;
  }

  // 重置所有状态
  joined.value = false;
  isScreenSharing.value = false;
  selectedAudioInput.value = '';
  selectedAudioOutput.value = '';
  activeRemoteUsers.value.clear();
  remoteStreams.value.clear();

  if (pc) { pc.close(); pc = null; }

  status.value = '已离开房间';
  console.log('✅ 房间离开流程完成');
}

// ---------- 设备控制 ----------
async function switchAudioDevice() {
  console.log('🎤 切换音频设备到:', selectedAudioInput.value);
}

async function switchAudioOutputDevice() {
  console.log('🔊 切换音频输出设备到:', selectedAudioOutput.value);
}

function toggleAudio() {
  if (localStream) {
    const audioTracks = localStream.getAudioTracks();
    audioTracks.forEach(track => {
      track.enabled = !track.enabled;
    });
    audioEnabled.value = !audioEnabled.value;
  }
}

function toggleVideo() {
  if (localStream) {
    const videoTracks = localStream.getVideoTracks();
    videoTracks.forEach(track => {
      track.enabled = !track.enabled;
    });
    videoEnabled.value = !videoEnabled.value;
  }
}

function toggleScreenShare() {
  if (isScreenSharing.value) {
    stopScreenCapture();
  } else {
    startScreenCapture();
  }
}

// 视频源切换功能
async function setVideoSource(source: 'camera' | 'screen' | 'both') {
  console.log('🔄 切换视频源到:', source);
  currentVideoSource.value = source;

  if (!joined.value) {
    console.log('⚠️ 尚未加入房间，无法切换视频源');
    return;
  }

  try {
    if (source === 'screen' || source === 'both') {
      if (!isScreenSharing.value) {
        await startScreenCapture();
      }
      if (source === 'screen' && localVideo.value) {
        // 只显示屏幕共享
        localVideo.value.srcObject = screenStream;
      }
    } else {
      if (isScreenSharing.value) {
        stopScreenCapture();
      }
      if (localVideo.value) {
        localVideo.value.srcObject = localStream;
      }
    }

    // 更新所有PeerConnection的视频流
    updateAllPeerConnectionsVideo();

  } catch (error) {
    console.error('❌ 切换视频源失败:', error);
    status.value = `切换视频源失败: ${error}`;
  }
}

// 更新所有PeerConnection的视频流
function updateAllPeerConnectionsVideo() {
  console.log('🔄 更新所有PeerConnection的视频流');

  peerConnections.value.forEach((pc, peerId) => {
    try {
      // 移除现有的视频发送器
      const senders = pc.getSenders();
      const videoSenders = senders.filter(sender =>
        sender.track && sender.track.kind === 'video'
      );

      videoSenders.forEach(sender => {
        pc.removeTrack(sender);
      });

      // 根据当前视频源添加新的视频轨道
      if (currentVideoSource.value === 'camera' && localStream) {
        const videoTracks = localStream.getVideoTracks();
        videoTracks.forEach(track => {
          pc.addTrack(track, localStream!);
        });
      } else if (currentVideoSource.value === 'screen' && screenStream) {
        const videoTracks = screenStream.getVideoTracks();
        videoTracks.forEach(track => {
          pc.addTrack(track, screenStream!);
        });
      } else if (currentVideoSource.value === 'both') {
        // 同时发送摄像头和屏幕共享
        if (localStream) {
          const videoTracks = localStream.getVideoTracks();
          videoTracks.forEach(track => {
            pc.addTrack(track, localStream!);
          });
        }
        if (screenStream) {
          const videoTracks = screenStream.getVideoTracks();
          videoTracks.forEach(track => {
            pc.addTrack(track, screenStream!);
          });
        }
      }

      console.log(`✅ 已更新 ${peerId} 的视频流`);
    } catch (error) {
      console.error(`❌ 更新 ${peerId} 视频流失败:`, error);
    }
  });
}

// 请求媒体设备权限
async function requestMediaPermissions() {
  console.log('🎤 用户手动请求媒体设备权限...');
  status.value = '正在请求媒体设备权限...';

  try {
    // 先枚举设备
    await enumerateDevices();

    // 尝试获取音频权限
    console.log('🎤 尝试获取音频权限...');
    const audioStream = await navigator.mediaDevices.getUserMedia({ audio: true, video: false });
    console.log('✅ 音频权限获取成功');
    audioStream.getTracks().forEach(track => track.stop());

    // 尝试获取视频权限
    console.log('📹 尝试获取视频权限...');
    const videoStream = await navigator.mediaDevices.getUserMedia({ audio: false, video: true });
    console.log('✅ 视频权限获取成功');
    videoStream.getTracks().forEach(track => track.stop());

    status.value = '✅ 媒体设备权限获取成功！现在可以加入房间了。';

    // 重新枚举设备以获取标签
    await enumerateDevices();

  } catch (error) {
    console.error('❌ 权限请求失败:', error);

    if (error instanceof Error) {
      switch (error.name) {
        case 'NotAllowedError':
          status.value = '❌ 权限被拒绝。请在系统设置中允许此应用访问摄像头和麦克风。';
          break;
        case 'NotFoundError':
          status.value = '❌ 未找到可用的摄像头或麦克风设备。';
          break;
        case 'NotReadableError':
          status.value = '❌ 摄像头或麦克风被其他应用占用。';
          break;
        default:
          status.value = `❌ 权限请求失败: ${error.message}`;
      }
    }
  }
}

// 切换开发者工具
function toggleDevTools() {
  // 检查是否在Tauri环境中
  const isTauri = typeof window !== 'undefined' && '__TAURI__' in window;

  if (isTauri) {
    console.log('🔧 Tauri环境 - 开发者工具在调试模式下自动打开');
    alert('在调试模式下，开发者工具已自动打开。\n如果未看到，请检查应用是否在调试模式下运行。\n快捷键: F12 或 Ctrl+Shift+I');
  } else {
    // 浏览器环境
    console.log('🔧 浏览器环境 - 使用快捷键提示');
    alert('浏览器环境请使用 F12 或 Ctrl+Shift+I 打开开发者工具');
  }
}

// 连接状态监控
function monitorConnectionState() {
  console.log('🔍 连接状态监控:', {
    joined: joined.value,
    room: room.value,
    user: userId.value,
    wsState: ws ? getWebSocketState(ws.readyState) : 'null',
    pcState: pc ? pc.connectionState : 'null',
    localStream: localStream ? 'active' : 'null',
    screenStream: screenStream ? 'active' : 'null',
    peerConnections: peerConnections.value.size
  });
}

// WebSocket 状态字符串
function getWebSocketState(state: number): string {
  switch (state) {
    case WebSocket.CONNECTING: return 'CONNECTING';
    case WebSocket.OPEN: return 'OPEN';
    case WebSocket.CLOSING: return 'CLOSING';
    case WebSocket.CLOSED: return 'CLOSED';
    default: return 'UNKNOWN';
  }
}

// ---------- 工具函数 ----------
function generateRoomName(): string {
  return 'room-' + Math.random().toString(36).slice(2, 8);
}

function generateUserName(): string {
  return 'user-' + Math.random().toString(36).slice(2, 6);
}

// 获取用户媒体流的备用方案 - 针对Tauri环境优化
async function requestUserMediaWithFallback(): Promise<MediaStream> {
  console.log('🔍 开始获取用户媒体设备...');
  console.log('🌍 当前环境信息:', {
    userAgent: navigator.userAgent,
    isTauri: typeof window !== 'undefined' && '__TAURI__' in window,
    hasMediaDevices: !!(navigator.mediaDevices && navigator.mediaDevices.getUserMedia)
  });

  // 检查 mediaDevices 是否可用
  if (!navigator.mediaDevices || !navigator.mediaDevices.getUserMedia) {
    console.error('❌ 当前环境不支持 getUserMedia');
    throw new Error('当前环境不支持摄像头和麦克风访问');
  }

  // 检查是否有可用的设备
  let devices: MediaDeviceInfo[] = [];
  let hasAudio = false;
  let hasVideo = false;

  try {
    devices = await navigator.mediaDevices.enumerateDevices();
    hasAudio = devices.some(device => device.kind === 'audioinput');
    hasVideo = devices.some(device => device.kind === 'videoinput');
    console.log('📱 可用设备检查:', {
      totalDevices: devices.length,
      hasAudioInput: hasAudio,
      hasVideoInput: hasVideo,
      devices: devices.map(d => ({
        kind: d.kind,
        label: d.label || '未知设备',
        deviceId: d.deviceId
      }))
    });

    if (!hasAudio && !hasVideo) {
      console.warn('⚠️ 未检测到任何音频或视频输入设备');
      // 在Tauri环境中，即使没有设备也继续尝试创建空流
      if (typeof window !== 'undefined' && '__TAURI__' in window) {
        console.log('🔄 Tauri环境检测到无设备，创建空媒体流...');
        return new MediaStream();
      }
    }
  } catch (error) {
    console.warn('⚠️ 无法枚举设备:', error);
  }

  const isTauri = typeof window !== 'undefined' && '__TAURI__' in window;
  console.log('🌍 Tauri环境检测:', isTauri);

  // 定义媒体约束的优先级顺序
  const constraintsList = isTauri ? [
    // Tauri环境：使用更宽松的约束
    { audio: true, video: false },  // 优先音频
    { audio: false, video: true },  // 其次视频
    { audio: true, video: true },   // 最后尝试两者
  ] : [
    // 浏览器环境：标准约束
    { audio: true, video: true },
    { audio: true, video: false },
    { audio: false, video: { width: 640, height: 480 } },
    { audio: true, video: { width: 320, height: 240 } }
  ];

  // Tauri环境的特殊处理
  if (isTauri) {
    console.log('🔄 Tauri环境：检查权限状态...');

    // 在Tauri中，先尝试获取权限
    try {
      const testStream = await navigator.mediaDevices.getUserMedia({ audio: true, video: false });
      console.log('✅ Tauri环境音频权限获取成功');
      testStream.getTracks().forEach(track => track.stop());
    } catch (error) {
      console.warn('⚠️ Tauri环境音频权限检查失败:', error);

      if (error instanceof Error && error.name === 'NotAllowedError') {
        // 权限被拒绝，显示用户友好的提示
        throw new Error('麦克风权限被拒绝。请在系统设置中允许此应用访问麦克风。');
      }
    }
  }

  // 尝试获取媒体流
  for (let i = 0; i < constraintsList.length; i++) {
    const constraints = constraintsList[i];
    console.log(`🎯 尝试获取媒体设备 (方案 ${i + 1}):`, constraints);

    try {
      const stream = await navigator.mediaDevices.getUserMedia(constraints);
      console.log(`✅ 方案 ${i + 1} 成功获取媒体流`);
      console.log('📹 获取到的音视频轨道:', {
        audioTracks: stream.getAudioTracks().length,
        videoTracks: stream.getVideoTracks().length,
        streamId: stream.id
      });

      // 验证轨道是否可用
      stream.getTracks().forEach(track => {
        console.log(`📟 轨道状态: ${track.kind} - enabled: ${track.enabled}, readyState: ${track.readyState}, id: ${track.id}, label: ${track.label}`);
      });

      // 在Tauri环境中，添加额外的处理
      if (isTauri) {
        console.log('🔄 Tauri环境：处理媒体流...');

        // 检查轨道是否真的在工作
        const audioTracks = stream.getAudioTracks();
        const videoTracks = stream.getVideoTracks();

        if (audioTracks.length > 0) {
          const audioTrack = audioTracks[0];
          console.log('🎤 Tauri音频轨道信息:', {
            enabled: audioTrack.enabled,
            muted: audioTrack.muted,
            readyState: audioTrack.readyState,
            label: audioTrack.label || 'Tauri麦克风'
          });
        }

        if (videoTracks.length > 0) {
          const videoTrack = videoTracks[0];
          console.log('📹 Tauri视频轨道信息:', {
            enabled: videoTrack.enabled,
            muted: videoTrack.muted,
            readyState: videoTrack.readyState,
            label: videoTrack.label || 'Tauri摄像头'
          });
        }
      }

      return stream;
    } catch (error) {
      console.warn(`⚠️ 方案 ${i + 1} 失败:`, error);

      // 详细的错误分析
      if (error instanceof Error) {
        console.error('❌ 错误详情:', {
          name: error.name,
          message: error.message,
          constraint: (error as any).constraint,
          toString: error.toString()
        });

        // Tauri环境的特殊错误处理
        if (isTauri) {
          switch (error.name) {
            case 'NotAllowedError':
              console.warn('🚫 Tauri权限被拒绝:', error.message);
              status.value = '权限被拒绝。请在系统设置中允许应用访问摄像头/麦克风。';
              break;
            case 'NotFoundError':
              console.warn('🔍 Tauri未找到设备:', error.message);
              status.value = '未找到可用的摄像头或麦克风。';
              break;
            case 'NotReadableError':
              console.warn('🔒 Tauri设备被占用:', error.message);
              status.value = '摄像头或麦克风被其他应用占用。';
              break;
            case 'OverconstrainedError':
              console.warn('⚖️ Tauri约束不满足:', error.message);
              status.value = '设备不满足要求，尝试降低质量设置。';
              break;
            default:
              console.warn('❓ Tauri未知错误:', error.message);
              status.value = `获取媒体设备失败: ${error.message}`;
          }
        }
      }

      if (i === constraintsList.length - 1) {
        // 所有方案都失败了
        if (isTauri) {
          console.log('🔄 Tauri环境所有方案失败，创建空媒体流继续...');
          status.value = '未获取到音视频设备，已加入房间（无音视频）';
          return new MediaStream();
        } else {
          throw error;
        }
      }
    }
  }

  // 理论上不会执行到这里
  console.log('🔄 创建默认空媒体流...');
  return new MediaStream();
}

// 处理媒体设备错误
function handleMediaError(error: any): void {
  console.error('❌ 媒体设备错误详情:', error);

  let errorMessage = '无法获取媒体设备';
  let suggestion = '';

  if (error.name === 'NotAllowedError') {
    errorMessage = '摄像头/麦克风权限被拒绝';
    suggestion = '请在浏览器设置中允许访问摄像头和麦克风';
  } else if (error.name === 'NotFoundError') {
    errorMessage = '未找到可用的摄像头或麦克风';
    suggestion = '请检查设备是否已连接并正常工作';
  } else if (error.name === 'NotReadableError') {
    errorMessage = '摄像头或麦克风被其他应用占用';
    suggestion = '请关闭其他可能正在使用摄像头的应用';
  } else if (error.name === 'OverconstrainedError') {
    errorMessage = '设备不满足要求的约束条件';
    suggestion = '尝试使用其他设备或降低质量要求';
  } else if (error.name === 'TypeError') {
    errorMessage = '媒体设备不可用';
    suggestion = '请检查设备连接和驱动程序';
  }

  console.log('💡 建议:', suggestion);
  status.value = `${errorMessage} - ${suggestion}`;

  // 尝试创建一个空的媒体流以便继续（只连接，不发送音视频）
  console.log('🔄 尝试创建空媒体流继续连接...');
  try {
    localStream = new MediaStream();
    status.value = '已连接（无音视频设备）';
  } catch (e) {
    console.error('❌ 创建空媒体流失败:', e);
  }
}

// 创建并发送 Offer（用于屏幕共享重新协商）
async function createAndSendOffer(pc: RTCPeerConnection, peerId: string) {
  try {
    console.log(`🔄 为 ${peerId} 创建新的 Offer...`);

    const offer = await pc.createOffer();
    await pc.setLocalDescription(offer);

    sendSignal({
      type: 'Offer',
      to: peerId,
      from: userId.value,
      sdp: offer.sdp
    });

    console.log(`✅ 新的 Offer 已发送给: ${peerId}`);
  } catch (error) {
    console.error(`❌ 为 ${peerId} 创建 Offer 失败:`, error);
  }
}

// 更新所有 PeerConnection 添加屏幕流
function updatePeerConnectionsWithScreenShare() {
  if (!screenStream) return;

  console.log('🖥️ 正在将屏幕流添加到所有 PeerConnection...');
  console.log('🖥️ 屏幕流轨道:', screenStream.getTracks().map(t => ({ kind: t.kind, id: t.id, enabled: t.enabled })));

  peerConnections.value.forEach((pc, peerId) => {
    try {
      console.log(`🖥️ 处理连接: ${peerId}`);

      // 获取当前的发送器
      const senders = pc.getSenders();
      console.log(`📤 当前发送器数量: ${senders.length}`);

      // 移除所有现有的屏幕轨道
      const screenSenders = senders.filter(sender => {
        const isScreenTrack = sender.track &&
          !localStream?.getTracks().includes(sender.track) &&
          sender.track.kind === 'video';
        if (isScreenTrack) {
          console.log(`🖥️ 移除现有屏幕轨道从: ${peerId}`);
        }
        return isScreenTrack;
      });

      screenSenders.forEach(sender => {
        pc.removeTrack(sender);
      });

      // 添加新的屏幕轨道到每个流
      screenStream!.getTracks().forEach(track => {
        console.log(`🖥️ 添加屏幕轨道到 ${peerId}: ${track.kind} (ID: ${track.id})`);
        pc.addTrack(track, screenStream!);

        // 触发重新协商
        if (pc.signalingState === 'stable') {
          console.log(`🔄 为屏幕共享触发重新协商: ${peerId}`);
          createAndSendOffer(pc, peerId);
        }
      });

      console.log(`✅ 屏幕轨道已更新到: ${peerId}`);
    } catch (error) {
      console.error(`❌ 更新 ${peerId} 屏幕轨道失败:`, error);
    }
  });
}

// 从所有 PeerConnection 中移除屏幕流
function removeScreenShareFromPeerConnections() {
  if (!screenStream) return;

  console.log('🖥️ 正在从所有 PeerConnection 中移除屏幕流...');

  // 在停止流之前获取轨道，避免screenStream变为null
  const screenTracks = screenStream.getTracks();

  peerConnections.value.forEach((pc, peerId) => {
    try {
      const senders = pc.getSenders();
      senders.forEach(sender => {
        if (sender.track && screenTracks.includes(sender.track)) {
          console.log(`🖥️ 移除屏幕轨道从: ${peerId}`);
          pc.removeTrack(sender);
        }
      });

      // 触发重新协商以通知远程端移除了屏幕共享
      if (pc.signalingState === 'stable') {
        console.log(`🔄 为移除屏幕共享触发重新协商: ${peerId}`);
        createAndSendOffer(pc, peerId);
      }
    } catch (error) {
      console.error(`❌ 从 ${peerId} 移除屏幕轨道失败:`, error);
    }
  });
}

// ---------- 生命周期 ----------
onMounted(async () => {
  // 获取服务器配置
  const config = await getServerConfig();
  if (config) {
    SIGNALING_WS = config.websocket_url;
  }

  // 自动获取设备和屏幕信息
  await enumerateDevices();
  await enumerateScreens();
});

onUnmounted(() => {
  leave();
});
</script>

<style scoped>
#controls {
  margin-bottom: 20px;
  padding: 15px;
  background-color: #f5f5f5;
  border-radius: 8px;
  display: flex;
  gap: 10px;
  align-items: center;
  flex-wrap: wrap;
}

#system-info {
  margin-bottom: 20px;
  padding: 15px;
  background-color: #e8f4fd;
  border-radius: 8px;
  border-left: 4px solid #007bff;
}

.info-section {
  margin-bottom: 15px;
}

.info-section h4 {
  margin-bottom: 8px;
  color: #333;
}

.device-item, .screen-item {
  padding: 8px 12px;
  margin: 4px 0;
  background-color: white;
  border-radius: 4px;
  border: 1px solid #ddd;
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.device-kind, .primary-badge {
  font-size: 12px;
  padding: 2px 6px;
  border-radius: 3px;
  background-color: #007bff;
  color: white;
}

#device-controls {
  margin-bottom: 20px;
  padding: 15px;
  background-color: #f0f8ff;
  border-radius: 8px;
  display: flex;
  gap: 15px;
  align-items: center;
  flex-wrap: wrap;
  border-left: 4px solid #28a745;
}

#device-controls label {
  display: flex;
  align-items: center;
  gap: 8px;
  font-weight: 500;
  color: #333;
}

#device-controls select {
  padding: 6px 10px;
  border: 1px solid #ddd;
  border-radius: 4px;
  font-size: 14px;
  min-width: 150px;
  background-color: white;
}

#controls input, #controls select {
  padding: 8px 12px;
  border: 1px solid #ddd;
  border-radius: 4px;
  font-size: 14px;
}

#controls button {
  padding: 8px 16px;
  border: none;
  border-radius: 4px;
  background-color: #007bff;
  color: white;
  cursor: pointer;
  font-size: 14px;
  transition: background-color 0.2s;
}

#controls button:hover:not(:disabled) {
  background-color: #0056b3;
}

#controls button:disabled {
  background-color: #6c757d;
  cursor: not-allowed;
}

#controls button:nth-child(4) {
  background-color: #28a745;
}

#controls button:nth-child(4):hover:not(:disabled) {
  background-color: #1e7e34;
}

#controls button:nth-child(7) {
  background-color: #ffc107;
  color: #212529;
}

#controls button:nth-child(7):hover:not(:disabled) {
  background-color: #e0a800;
}

#controls button:nth-child(8) {
  background-color: #dc3545;
}

#controls button:nth-child(8):hover:not(:disabled) {
  background-color: #c82333;
}

#controls button:nth-child(9) {
  background-color: #ffc107;
  color: #212529;
}

#controls button:nth-child(9):hover:not(:disabled) {
  background-color: #e0a800;
}

#controls button:nth-child(10) {
  background-color: #17a2b8;
  color: white;
}

#controls button:nth-child(10):hover:not(:disabled) {
  background-color: #138496;
}

.permissions-btn {
  background-color: #ffc107 !important;
  color: #212529 !important;
  border: 1px solid #e0a800 !important;
  font-weight: bold !important;
}

.permissions-btn:hover:not(:disabled) {
  background-color: #e0a800 !important;
  transform: translateY(-1px);
}

.permissions-btn:disabled {
  background-color: #6c757d !important;
  color: white !important;
  transform: none;
}

.debug-btn {
  background-color: #6c757d !important;
  color: white !important;
  border: 1px solid #5a6268 !important;
}

.debug-btn:hover:not(:disabled) {
  background-color: #5a6268 !important;
}

.video-container {
  margin-bottom: 20px;
}

.video-container h3 {
  margin-bottom: 10px;
  color: #333;
}

#remote-videos {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
  min-height: 200px;
}

video {
  width: 45%;
  max-width: 400px;
  border: 2px solid #ddd;
  border-radius: 8px;
  margin: 4px;
  background-color: #000;
  box-shadow: 0 2px 4px rgba(0,0,0,0.1);
}

#localVideo {
  width: 300px;
  height: 225px;
}

.video-source-toggle {
  margin-top: 10px;
  display: flex;
  gap: 8px;
  justify-content: center;
}

.video-source-toggle button {
  padding: 6px 12px;
  border: 1px solid #007bff;
  border-radius: 4px;
  background-color: white;
  color: #007bff;
  cursor: pointer;
  font-size: 12px;
  transition: all 0.2s;
  flex: 1;
  max-width: 100px;
}

.video-source-toggle button:hover:not(:disabled) {
  background-color: #e6f3ff;
  transform: translateY(-1px);
}

.video-source-toggle button.active {
  background-color: #007bff;
  color: white;
  border-color: #0056b3;
  box-shadow: 0 2px 4px rgba(0,123,255,0.3);
}

.video-source-toggle button:disabled {
  background-color: #f8f9fa;
  color: #6c757d;
  border-color: #dee2e6;
  cursor: not-allowed;
  transform: none;
  box-shadow: none;
}

audio {
  display: block;
  width: 100%;
  margin-top: 8px;
  border-radius: 4px;
  box-shadow: 0 1px 3px rgba(0,0,0,0.1);
}

p {
  padding: 10px;
  background-color: #e9ecef;
  border-radius: 4px;
  margin-top: 15px;
  font-weight: 500;
}

@media (max-width: 768px) {
  #controls {
    flex-direction: column;
    align-items: stretch;
  }

  video {
    width: 100%;
    max-width: none;
  }
}
</style>