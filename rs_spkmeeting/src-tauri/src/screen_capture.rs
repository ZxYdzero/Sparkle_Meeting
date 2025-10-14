use serde::{Deserialize, Serialize};
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenInfo {
    pub id: String,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub is_primary: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureOptions {
    pub screen_id: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub frame_rate: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureStream {
    pub stream_id: String,
    pub width: u32,
    pub height: u32,
    pub format: String,
}

#[derive(Clone)]
pub struct ScreenCaptureState {
    pub active_captures: Arc<Mutex<std::collections::HashMap<String, CaptureSession>>>,
}

// 确保ScreenCaptureState满足Tauri状态管理要求的trait
unsafe impl Send for ScreenCaptureState {}
unsafe impl Sync for ScreenCaptureState {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureSession {
    pub stream_id: String,
    pub is_active: bool,
    pub frame_count: u64,
}

#[tauri::command]
pub async fn enumerate_screens() -> Result<Vec<ScreenInfo>, String> {
    match get_available_screens().await {
        Ok(screens) => Ok(screens),
        Err(e) => Err(format!("Failed to enumerate screens: {}", e)),
    }
}

#[tauri::command]
pub async fn start_screen_capture(
    options: CaptureOptions,
    state: tauri::State<'_, ScreenCaptureState>,
) -> Result<CaptureStream, String> {
    match start_capture_session(options, state).await {
        Ok(stream) => Ok(stream),
        Err(e) => Err(format!("Failed to start screen capture: {}", e)),
    }
}

#[tauri::command]
pub async fn stop_screen_capture(
    stream_id: String,
    state: tauri::State<'_, ScreenCaptureState>,
) -> Result<bool, String> {
    match stop_capture_session(stream_id, state).await {
        Ok(stopped) => Ok(stopped),
        Err(e) => Err(format!("Failed to stop screen capture: {}", e)),
    }
}

#[tauri::command]
pub async fn get_capture_status(
    stream_id: String,
    state: tauri::State<'_, ScreenCaptureState>,
) -> Result<Option<CaptureSession>, String> {
    let captures = state.active_captures.lock().await;
    Ok(captures.get(&stream_id).cloned())
}

async fn get_available_screens() -> Result<Vec<ScreenInfo>> {
    let mut screens = Vec::new();

    #[cfg(target_os = "windows")]
    {
        windows::enumerate_screens(&mut screens).await?;
    }

    #[cfg(target_os = "macos")]
    {
        macos::enumerate_screens(&mut screens).await?;
    }

    #[cfg(target_os = "linux")]
    {
        linux::enumerate_screens(&mut screens).await?;
    }

    Ok(screens)
}

async fn start_capture_session(
    options: CaptureOptions,
    state: tauri::State<'_, ScreenCaptureState>,
) -> Result<CaptureStream> {
    let stream_id = uuid::Uuid::new_v4().to_string();

    // 获取屏幕信息
    let screens = get_available_screens().await?;
    let target_screen = if let Some(screen_id) = options.screen_id {
        screens.into_iter().find(|s| s.id == screen_id)
            .ok_or_else(|| anyhow::anyhow!("Screen not found"))?
    } else {
        screens.into_iter()
            .find(|s| s.is_primary)
            .ok_or_else(|| anyhow::anyhow!("No primary screen found"))?
    };

    let width = options.width.unwrap_or(target_screen.width);
    let height = options.height.unwrap_or(target_screen.height);

    // 启动捕获会话
    #[cfg(target_os = "windows")]
    {
        windows::start_capture(&stream_id, &target_screen, width, height, options.frame_rate).await?;
    }

    #[cfg(target_os = "macos")]
    {
        macos::start_capture(&stream_id, &target_screen, width, height, options.frame_rate).await?;
    }

    #[cfg(target_os = "linux")]
    {
        linux::start_capture(&stream_id, &target_screen, width, height, options.frame_rate).await?;
    }

    // 记录活动捕获
    let session = CaptureSession {
        stream_id: stream_id.clone(),
        is_active: true,
        frame_count: 0,
    };

    let mut captures = state.active_captures.lock().await;
    captures.insert(stream_id.clone(), session);

    Ok(CaptureStream {
        stream_id,
        width,
        height,
        format: "rgba".to_string(),
    })
}

async fn stop_capture_session(
    stream_id: String,
    state: tauri::State<'_, ScreenCaptureState>,
) -> Result<bool> {
    // 停止平台特定的捕获
    #[cfg(target_os = "windows")]
    {
        windows::stop_capture(&stream_id).await?;
    }

    #[cfg(target_os = "macos")]
    {
        macos::stop_capture(&stream_id).await?;
    }

    #[cfg(target_os = "linux")]
    {
        linux::stop_capture(&stream_id).await?;
    }

    // 从活动捕获中移除
    let mut captures = state.active_captures.lock().await;
    Ok(captures.remove(&stream_id).is_some())
}

// 平台特定实现
#[cfg(target_os = "windows")]
mod windows {
    use crate::screen_capture::ScreenInfo;
    use anyhow::Result;

    pub async fn enumerate_screens(_screens: &mut Vec<ScreenInfo>) -> Result<()> {
        // 使用 Windows API 枚举显示器
        // 例如：使用 EnumDisplayMonitors 或 DXGI

        // 示例：添加一个主显示器
        _screens.push(ScreenInfo {
            id: "monitor-0".to_string(),
            name: "主显示器".to_string(),
            width: 1920,
            height: 1080,
            is_primary: true,
        });

        Ok(())
    }

    pub async fn start_capture(
        _stream_id: &str,
        _screen: &ScreenInfo,
        _width: u32,
        _height: u32,
        _frame_rate: Option<u32>,
    ) -> Result<()> {
        // 使用 Windows Graphics Capture API 或其他方法
        // 开始屏幕捕获
        Ok(())
    }

    pub async fn stop_capture(_stream_id: &str) -> Result<()> {
        // 停止屏幕捕获
        Ok(())
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use crate::screen_capture::ScreenInfo;
    use anyhow::Result;

    pub async fn enumerate_screens(_screens: &mut Vec<ScreenInfo>) -> Result<()> {
        // 使用 Core Graphics API 枚举显示器
        Ok(())
    }

    pub async fn start_capture(
        _stream_id: &str,
        _screen: &ScreenInfo,
        _width: u32,
        _height: u32,
        _frame_rate: Option<u32>,
    ) -> Result<()> {
        // 使用 Screen Capture Kit (macOS 12.3+) 或其他方法
        Ok(())
    }

    pub async fn stop_capture(_stream_id: &str) -> Result<()> {
        // 停止屏幕捕获
        Ok(())
    }
}

#[cfg(target_os = "linux")]
mod linux {
    use crate::screen_capture::ScreenInfo;
    use anyhow::Result;

    pub async fn enumerate_screens(_screens: &mut Vec<ScreenInfo>) -> Result<()> {
        // 使用 X11 或 Wayland API 枚举显示器
        Ok(())
    }

    pub async fn start_capture(
        _stream_id: &str,
        _screen: &ScreenInfo,
        _width: u32,
        _height: u32,
        _frame_rate: Option<u32>,
    ) -> Result<()> {
        // 使用 Pipewire 或 X11 捕获
        Ok(())
    }

    pub async fn stop_capture(_stream_id: &str) -> Result<()> {
        // 停止屏幕捕获
        Ok(())
    }
}