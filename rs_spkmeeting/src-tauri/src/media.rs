use serde::{Deserialize, Serialize};
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaDevice {
    pub device_id: String,
    pub label: String,
    pub kind: String, // "audioinput", "audiooutput", "videoinput"
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MediaStreamOptions {
    pub audio: bool,
    pub video: bool,
    pub audio_device_id: Option<String>,
    pub video_device_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MediaStreamInfo {
    pub stream_id: String,
    pub audio_tracks: Vec<String>,
    pub video_tracks: Vec<String>,
}

#[tauri::command]
pub async fn enumerate_media_devices() -> Result<Vec<MediaDevice>, String> {
    // 这里将实现媒体设备枚举
    // 由于 Tauri 在前端调用 WebRTC API 更方便，这里提供系统级别的设备信息
    match get_system_media_devices().await {
        Ok(devices) => Ok(devices),
        Err(e) => Err(format!("Failed to enumerate devices: {}", e)),
    }
}

#[tauri::command]
pub async fn get_user_media(options: MediaStreamOptions) -> Result<MediaStreamInfo, String> {
    // 这个函数将帮助获取媒体流
    match get_media_stream(options).await {
        Ok(stream_info) => Ok(stream_info),
        Err(e) => Err(format!("Failed to get media stream: {}", e)),
    }
}

async fn get_system_media_devices() -> Result<Vec<MediaDevice>> {
    let mut devices = Vec::new();

    // 在实际实现中，这里会调用系统API获取设备列表
    // 由于 WebRTC API 在前端更容易使用，这里主要用于提供系统级别的设备信息

    #[cfg(target_os = "windows")]
    {
        // Windows 特定的设备枚举
        windows::enumerate_devices(&mut devices)?;
    }

    #[cfg(target_os = "macos")]
    {
        // macOS 特定的设备枚举
        macos::enumerate_devices(&mut devices)?;
    }

    #[cfg(target_os = "linux")]
    {
        // Linux 特定的设备枚举
        linux::enumerate_devices(&mut devices)?;
    }

    Ok(devices)
}

async fn get_media_stream(_options: MediaStreamOptions) -> Result<MediaStreamInfo> {
    // 在实际实现中，这里会启动媒体流捕获
    // 由于大部分 WebRTC 操作在前端，这里主要用于辅助功能

    let stream_id = uuid::Uuid::new_v4().to_string();

    Ok(MediaStreamInfo {
        stream_id,
        audio_tracks: vec![],
        video_tracks: vec![],
    })
}

// 平台特定模块
#[cfg(target_os = "windows")]
mod windows {
    use crate::media::MediaDevice;
    use anyhow::{anyhow, Result};
    use std::ptr;
    use windows::{
        Win32::{
            Media::Audio::{
                IMMDeviceEnumerator, MMDeviceEnumerator,
                DEVICE_STATE_ACTIVE, EDataFlow,
            },
            System::Com::{
                CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_ALL, COINIT_MULTITHREADED,
                STGM_READ,
            },
        },
    };

    pub fn enumerate_devices(devices: &mut Vec<MediaDevice>) -> Result<()> {
        unsafe {
            // 初始化 COM
            let _init_result = CoInitializeEx(Some(ptr::null()), COINIT_MULTITHREADED);
            if _init_result.is_err() {
                return Err(anyhow!("CoInitializeEx failed"));
            }

            // 创建设备枚举器
            let enumerator: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)
                .map_err(|e| anyhow!("CoCreateInstance failed: {e}"))?;

            // ==== 枚举扬声器 ====
            let speakers = enumerate_device_type(&enumerator, EDataFlow(0))?; // eRender = 0
            devices.extend(speakers);

            // ==== 枚举麦克风 ====
            let mics = enumerate_device_type(&enumerator, EDataFlow(1))?; // eCapture = 1
            devices.extend(mics);

            CoUninitialize();
        }
        Ok(())
    }

    unsafe fn enumerate_device_type(
        enumerator: &IMMDeviceEnumerator,
        data_flow: EDataFlow,
    ) -> Result<Vec<MediaDevice>> {
        let mut list = Vec::new();
        let collection = enumerator
            .EnumAudioEndpoints(data_flow, DEVICE_STATE_ACTIVE)
            .map_err(|e| anyhow!("EnumAudioEndpoints failed: {e}"))?;

        let count = collection.GetCount().unwrap_or(0);
        for i in 0..count {
            if let Ok(device) = collection.Item(i) {
                // 简化设备名称获取方式
                let id = device
                    .GetId()
                    .ok()
                    .and_then(|s| s.to_string().ok())
                    .unwrap_or_else(|| "UnknownID".to_string());

                let friendly_name = format!("Audio Device {}", i + 1);

                let kind = match data_flow.0 {
                    0 => "audiooutput", // eRender
                    1 => "audioinput",  // eCapture
                    _ => "unknown",
                };

                list.push(MediaDevice {
                    device_id: id,
                    label: friendly_name,
                    kind: kind.to_string(),
                });
            }
        }
        Ok(list)
    }
}



#[cfg(target_os = "macos")]
mod macos {
    use crate::media::MediaDevice;
    use anyhow::Result;

    pub fn enumerate_devices(_devices: &mut Vec<MediaDevice>) -> Result<()> {
        // TODO
        Ok(())
    }
}

#[cfg(target_os = "linux")]
mod linux {
    use crate::media::MediaDevice;
    use anyhow::Result;

    pub fn enumerate_devices(_devices: &mut Vec<MediaDevice>) -> Result<()> {
        // TODO
        Ok(())
    }
}