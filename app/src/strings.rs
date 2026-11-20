//! UI copy in English and Simplified Chinese, chosen from the system language.
//!
//! Keys match `locales/app.yml` so `nsh package` and this table stay aligned.

use nsh_core::prelude::*;

fn is_zh() -> bool {
    let tag = locale().to_string();
    tag.starts_with("zh")
}

/// Look up a key; fills `%{name}` style placeholders from `args`.
pub fn tr(key: &str, args: &[(&str, &str)]) -> String {
    let key = if cfg!(target_os = "android") {
        match key {
            "connecting.wifi_sub" => "android.connecting.wifi_sub",
            "cam.step_wifi_sub" => "android.cam.step_wifi_sub",
            "cam.step_pair_gr3" => "android.cam.step_pair_gr3",
            "error.bt_body" => "android.error.bt_body",
            "error.wifi_body" => "android.error.wifi_body",
            "conn.local_network.body" => "android.conn.local_network.body",
            "conn.wifi_config.body" => "android.conn.wifi_config.body",
            _ => key,
        }
    } else {
        key
    };
    let mut text = raw(key).to_owned();
    for (name, value) in args {
        text = text.replace(&format!("%{{{name}}}"), value);
    }
    text
}

fn raw(key: &str) -> &'static str {
    if is_zh() {
        zh(key).unwrap_or_else(|| en(key))
    } else {
        en(key)
    }
}

fn en(key: &str) -> &'static str {
    match key {
        "android.connecting.wifi_sub" => {
            "Choose the camera in the Android network prompt. Stay connected even without internet."
        }
        "android.error.bt_body" => {
            "Turn on Bluetooth. In Android Settings, allow Nearby devices for GRKit on Android 12 or later. On Android 11 or earlier, allow Location and turn on Location services."
        }
        "android.error.wifi_body" => {
            "Select the camera in the network prompt. On Android 13 or later, allow Nearby devices for GRKit. On Android 12 or earlier, allow Location and turn on Location services."
        }

        "error.cancel_pending" => "Cancel Pending Update",
        "software.checking" => "Checking for updates…",
        "software.check" => "Check for Updates",
        "error.download_title" => "Could Not Prepare the Update",
        "error.download_body" => {
            "Could not download the update. Check the internet connection and try again."
        }
        "error.service_title" => "Could Not Complete the Request",
        "installed.verified" => "The camera is running this version.",
        "sending.preparing" => "Preparing and sending package…",
        "software.unavailable" => "Update information unavailable",
        "loading.camera" => "Connecting to your camera…",
        "loading.update" => "Checking for updates…",
        "tab.camera" => "Camera",
        "tab.settings" => "Settings",
        "home.title" => "Add Your Camera",
        "home.subtitle" => "Connect your camera. Manage GRKit.",
        "home.add" => "Add Camera",
        "home.footnote" => "Needs GRKit 1.7.0 or later on the camera.",
        "error.old_title" => "GRKit Needs an Update",
        "error.old_body" => "Install GRKit 1.7.0 or later on the card first.",
        "nearby.powered_on" => "Powered on",
        "nearby.hint" => {
            "First time with this camera? On the camera, select Bluetooth Setting → Action Mode On → Pairing → Execute Pairing"
        }
        "pair.hint_gr3" => {
            "First time with this camera? On the camera, select Bluetooth Setting → Action Mode On → Pairing → Execute Pairing"
        }
        "pair.hint_gr4" => {
            "First time with this camera? On the camera, select Wireless Communication → Action Mode On → Pairing → Execute Pairing"
        }
        "nearby.connect" => "Connect",
        "nearby.not_now" => "Not Now",
        "scan.title" => "Add a Camera",
        "scan.subtitle" => "Keep it close by, with Bluetooth on.",
        "scan.looking" => "Looking for cameras",
        "scan.nearby_off" => "Standby",
        "scan.hint" => {
            "First time? On the camera: Bluetooth Setting → Action Mode On → Pairing → Execute Pairing"
        }
        "scan.close" => "Close",
        "scan.back" => "Back to Cameras",
        "connecting.cancel" => "Cancel",
        "connecting.title" => "Connecting to %{model}",
        "connecting.keep_on" => "Keep the camera on and nearby.",
        "connecting.bt" => "Connect over Bluetooth",
        "connecting.wifi" => "Join the camera's Wi-Fi",
        "connecting.wifi_sub" => "If iOS asks, allow it.",
        "connecting.version" => "Read GRKit version",
        "connected.status" => "Connected",
        "connected.offline" => "Not Connected",
        "connected.reconnecting" => "Reconnecting…",
        "connected.whats_new" => "What's New",
        "connected.update" => "Update",
        "connected.grkit" => "GRKit",
        "connected.firmware" => "Firmware",
        "connected.up_to_date" => "GRKit Is Up to Date",
        "connected.software_update" => "Software Update",
        "software.title" => "Software Update",
        "software.installed" => "Installed",
        "software.looks_stay" => "Your looks and settings stay.",
        "software.update_now" => "Update Now",
        "software.confirm_note" => "Confirm on the camera, then restart it.",
        "software.up_to_date" => "GRKit is up to date",
        "software.learn_more" => "Learn More…",
        "sending.title" => "Updating %{model}",
        "sending.progress" => "Sending to your camera…",
        "sending.step_send" => "Send to camera",
        "sending.step_confirm" => "Confirm on camera",
        "sending.step_restart" => "Restart camera",
        "sending.keep_on" => "Keep the camera on.",
        "sending.cancel" => "Cancel",
        "confirm.title" => "Confirm on the Camera",
        "confirm.subtitle" => "The update is on the camera and verified.",
        "confirm.step1" => "MENU → About GRKit",
        "confirm.step2" => "Choose Install",
        "confirm.step3" => "Choose Execute",
        "confirm.waiting" => "Waiting for the camera · %{time}",
        "restart.title" => "Restart the Camera",
        "restart.subtitle" => {
            "Turn the camera off and on, then reconnect to verify the installed version."
        }
        "restart.done" => "Check Installed Version",
        "restart.cancel" => "Cancel Update",
        "installed.title" => "GRKit %{version} Is Installed",
        "installed.done" => "Done",
        "error.card_full_title" => "Card Is Full",
        "error.card_full_body" => "Free %{size}, then try again.",
        "error.not_confirmed_title" => "Not Confirmed",
        "error.not_confirmed_body" => {
            "Confirmation timed out. Check the camera before trying again."
        }
        "error.refused_title" => "Update Refused",
        "error.refused_body" => {
            "The camera refused this package. Check that it supports your camera and firmware."
        }
        "error.lost_title" => "Connection Lost",
        "error.lost_body" => {
            "Reconnect to check the update status. The camera may still have a pending update."
        }
        "error.connect_lost_body" => "Keep the camera on and close to your phone, then try again.",
        "error.pair_title" => "Pairing Needed",
        "error.pair_body" => "Complete pairing on the camera, then try again.",
        "error.wifi_title" => "Could Not Join Wi-Fi",
        "error.wifi_body" => "Allow the network prompt, keep the camera nearby, then try again.",
        "error.card_ready_title" => "Camera Not Ready",
        "error.card_ready_body" => "The memory card is still starting. Try again in a moment.",
        "error.bt_title" => "Bluetooth Unavailable",
        "error.bt_body" => "Turn on Bluetooth and allow access for GRKit.",
        "error.try_again" => "Try Again",
        "error.send_again" => "Send Again",
        "error.close" => "Close",
        "settings.title" => "Settings",
        "settings.about" => "About",
        "settings.version" => "Version",
        "settings.rate" => "Rate GRKit",
        "settings.privacy" => "Privacy Policy",
        "settings.more_maker" => "More from the Maker",
        "settings.gr_snap_name" => "GR Snap",
        "settings.gr_snap_desc" => "Wireless transfer and remote for RICOH GR",
        "settings.view" => "View",
        _ => crate::strings_looks::en(key)
            .or_else(|| crate::strings_connect::en(key))
            .unwrap_or("???"),
    }
}

fn zh(key: &str) -> Option<&'static str> {
    Some(match key {
        "android.connecting.wifi_sub" => {
            "请在 Android 网络提示中选择相机。即使没有互联网，也请保持连接。"
        }
        "android.error.bt_body" => {
            "请打开蓝牙。在 Android 设置中，Android 12 及更新版本需允许 GRKit 使用“附近的设备”；Android 11 及更早版本需允许位置权限，并打开定位服务。"
        }
        "android.error.wifi_body" => {
            "请在网络提示中选择相机。Android 13 及更新版本需允许 GRKit 使用“附近的设备”；Android 12 及更早版本需允许位置权限，并打开定位服务。"
        }
        "error.cancel_pending" => "取消待处理的更新",
        "software.checking" => "正在检查更新…",
        "software.check" => "检查更新",
        "software.unavailable" => "暂时无法获取更新信息",
        "sending.preparing" => "正在准备并发送更新包…",
        "installed.verified" => "已确认相机正在运行此版本。",
        "error.service_title" => "无法完成请求",
        "error.download_title" => "无法准备更新",
        "error.download_body" => "无法下载更新。请检查网络连接后重试。",
        "loading.camera" => "正在连接相机…",
        "loading.update" => "正在检查更新…",
        "tab.camera" => "相机",
        "tab.settings" => "设置",
        "home.title" => "添加你的相机",
        "home.subtitle" => "连接相机，管理 GRKit。",
        "home.add" => "添加相机",
        "home.footnote" => "相机上需要 GRKit 1.7.0 或更高版本。",
        "error.old_title" => "需要更新 GRKit",
        "error.old_body" => "请先在卡上安装 GRKit 1.7.0 或更高版本。",
        "nearby.powered_on" => "已开机",
        "nearby.hint" => "首次连接？请在相机上选择“蓝牙设置 → 打开动作模式 → 配对 → 执行配对”",
        "pair.hint_gr3" => "首次连接？请在相机上选择“蓝牙设置 → 打开动作模式 → 配对 → 执行配对”",
        "pair.hint_gr4" => "首次连接？请在相机上选择“无线通信 → 打开动作模式 → 配对 → 执行配对”",
        "nearby.connect" => "连接",
        "nearby.not_now" => "暂不",
        "scan.title" => "添加相机",
        "scan.subtitle" => "请把相机放在附近，并开启相机蓝牙。",
        "scan.looking" => "正在查找相机",
        "scan.nearby_off" => "待机",
        "scan.hint" => "首次连接？在相机上：蓝牙设置 → 打开动作模式 → 配对 → 执行配对",
        "scan.close" => "关闭",
        "scan.back" => "返回相机列表",
        "connecting.cancel" => "取消",
        "connecting.title" => "正在连接 %{model}",
        "connecting.keep_on" => "请保持相机开机并在附近。",
        "connecting.bt" => "通过蓝牙连接",
        "connecting.wifi" => "加入相机的 Wi-Fi",
        "connecting.wifi_sub" => "若系统询问，请允许。",
        "connecting.version" => "读取 GRKit 版本",
        "connected.status" => "已连接",
        "connected.offline" => "未连接",
        "connected.reconnecting" => "正在重新连接…",
        "connected.whats_new" => "新功能",
        "connected.update" => "更新",
        "connected.grkit" => "GRKit",
        "connected.firmware" => "固件",
        "connected.up_to_date" => "GRKit 已是最新",
        "connected.software_update" => "软件更新",
        "software.title" => "软件更新",
        "software.installed" => "已安装",
        "software.looks_stay" => "你的胶片风格和设置会保留。",
        "software.update_now" => "立即更新",
        "software.confirm_note" => "请在相机上确认，然后重启相机。",
        "software.up_to_date" => "GRKit 已是最新",
        "software.learn_more" => "了解更多…",
        "sending.title" => "正在更新 %{model}",
        "sending.progress" => "正在发送到相机…",
        "sending.step_send" => "发送到相机",
        "sending.step_confirm" => "在相机上确认",
        "sending.step_restart" => "重启相机",
        "sending.keep_on" => "请保持相机开机。",
        "sending.cancel" => "取消",
        "confirm.title" => "在相机上确认",
        "confirm.subtitle" => "更新已在相机上并通过校验。",
        "confirm.step1" => "菜单 → 关于 GRKit",
        "confirm.step2" => "选择安装",
        "confirm.step3" => "选择执行",
        "confirm.waiting" => "等待相机 · %{time}",
        "restart.title" => "重启相机",
        "restart.subtitle" => "请将相机关机再开机，然后重新连接以确认已安装的版本。",
        "restart.done" => "检查已安装版本",
        "restart.cancel" => "取消更新",
        "installed.title" => "GRKit %{version} 已安装",
        "installed.done" => "完成",
        "error.card_full_title" => "存储卡已满",
        "error.card_full_body" => "请先腾出 %{size}，然后重试。",
        "error.not_confirmed_title" => "未确认",
        "error.not_confirmed_body" => "确认已超时。请先检查相机，再重试。",
        "error.refused_title" => "更新被拒绝",
        "error.refused_body" => "相机拒绝了此更新包。请确认更新包支持你的相机和固件版本。",
        "error.lost_title" => "连接已断开",
        "error.lost_body" => "请重新连接以检查更新状态。相机上可能仍有待处理的更新。",
        "error.connect_lost_body" => "请让相机保持开机并靠近手机，然后重试。",
        "error.pair_title" => "需要配对",
        "error.pair_body" => "请先在相机上完成配对，然后重试。",
        "error.wifi_title" => "无法加入 Wi-Fi",
        "error.wifi_body" => "请允许系统网络提示，保持相机在附近，然后重试。",
        "error.card_ready_title" => "相机未就绪",
        "error.card_ready_body" => "存储卡仍在启动，请稍后再试。",
        "error.bt_title" => "蓝牙不可用",
        "error.bt_body" => "请打开蓝牙并允许 GRKit 使用。",
        "error.try_again" => "重试",
        "error.send_again" => "重新发送",
        "error.close" => "关闭",
        "settings.title" => "设置",
        "settings.about" => "关于",
        "settings.version" => "版本",
        "settings.rate" => "为 GRKit 评分",
        "settings.privacy" => "隐私政策",
        "settings.more_maker" => "来自同一作者",
        "settings.gr_snap_name" => "GR Snap",
        "settings.gr_snap_desc" => "RICOH GR 的无线传输与遥控",
        "settings.view" => "查看",
        _ => {
            return crate::strings_looks::zh(key).or_else(|| crate::strings_connect::zh(key));
        }
    })
}

/// Format bytes as a short size label for errors (`12 MB`).
pub fn format_bytes(bytes: u64) -> String {
    const MB: u64 = 1024 * 1024;
    if bytes >= MB {
        format!("{} MB", (bytes + MB / 2) / MB)
    } else if bytes >= 1024 {
        format!("{} KB", (bytes + 512) / 1024)
    } else {
        format!("{bytes} B")
    }
}
