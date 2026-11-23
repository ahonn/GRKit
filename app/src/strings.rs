//! UI copy in English and Simplified Chinese, chosen from the system language.
//!
//! Keys match `locales/app.yml` so `nsh package` and this table stay aligned.

use nsh_core::prelude::*;

pub fn is_zh() -> bool {
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
            "app.store" => "android.app.store",
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
        "update.title" => "Updating %{model}",
        "update.step_download" => "Download update",
        "update.step_send" => "Send to camera",
        "update.step_confirm" => "Confirm on camera",
        "update.step_restart" => "Restart camera",
        "update.keep_on" => "Keep the camera on and stay in the app.",
        "update.download_body" => {
            "On the camera's Wi‑Fi, the phone downloads updates over cellular data. Check it and try again."
        }
        "update.cancel" => "Cancel Update",
        "update.cancelling" => "Cancelling…",
        "cam.more" => "More",
        "cam.auto_connecting" => "Looking for your camera…",
        "update.cancel_title" => "Cancel the Update?",
        "update.cancel_body_send" => "What was sent is removed from the camera.",
        "update.cancel_body_confirm" => "The update waiting on the camera is removed.",
        "update.cancel_body_restart" => "The camera won't install this update when it restarts.",
        "update.cancel_keep" => "Keep Updating",
        "update.cancel_confirm" => "Cancel Update",
        "update.reconnect" => "Reconnect",
        "update.ok" => "OK",
        "update.menu_value" => "Update %{version}",
        "update.updated_toast" => "GRKit updated to %{version}",
        "update.failed_title" => "Update Didn't Finish",
        "update.failed_sub" => {
            "The camera started without finishing the install, so GRKit isn't running."
        }
        "update.failed_body" => {
            "Restart the camera once more. If GRKit still doesn't start, put the card in a computer and install GRKit again at grkit.app."
        }
        "update.open_site" => "Open grkit.app",
        "restart.running_title" => "Restarting the Camera",
        "restart.running_body" => "Don't turn off the camera or leave the app.",
        "restart.manual_body" => "Turn the camera off and on, then tap Reconnect.",
        "restart.auto_body" => "The app turns the camera off and on, then checks the version.",
        "restart.run" => "Restart Camera",
        "restart.title" => "Restart the Camera",
        "restart.subtitle" => "The update is confirmed. It installs when the camera restarts.",
        "confirm.subtitle" => "The update is on the camera and waits for you.",
        "confirm.step1" => "Press ADJ and move to GRK",
        "confirm.step2" => "Choose MOD, then %{update}",
        "confirm.step3" => "Choose Execute",
        "confirm.waiting" => "Waiting · %{time} left",
        "installed.verified" => "The camera is running the new version.",
        "error.download_title" => "Couldn't Download the Update",
        "error.module_title" => "Update with the Memory Card",
        "error.module_body" => {
            "This update doesn't have the module %{module} that the camera has. Update GRKit with the memory card."
        }
        "error.package_title" => "Can't Prepare the Update",
        "error.package_body" => {
            "The files of this update are incomplete. Check for updates again later."
        }
        "error.card_full_title" => "Card Is Full",
        "error.card_full_body" => {
            "%{size} more is needed. Delete some photos on the camera, then try again."
        }
        "error.not_confirmed_title" => "Not Confirmed on the Camera",
        "error.not_confirmed_body" => {
            "No confirmation within 5 minutes, so the camera removed this update."
        }
        "error.refused_body" => "This update doesn't fit the firmware of this camera.",
        "error.lost_body" => "Reconnect, and the app continues with this update.",
        "software.confirm_note" => {
            "You confirm the update on the camera, then restart it once. Your film looks and settings stay."
        }
        "software.up_to_date" => "GRKit is up to date",
        "software.check_failed" => "Couldn't check for updates",
        "software.check_failed_body" => {
            "On the camera's Wi‑Fi, the phone downloads updates over cellular data. Check it and try again."
        }
        "software.retry" => "Try Again",
        "software.current" => "Current %{version} · %{model}",
        "software.update" => "Update",
        "software.latest_sub" => "Up to Date · %{model}",
        "software.whats_new" => "What's New",
        "software.whats_new_here" => "What's New in This Version",
        "software.show_all" => "Show All",
        "software.history" => "Update History",
        "software.history_github" => "View on GitHub",
        "software.firmware_old" => "Camera firmware needs an update",
        "software.firmware_old_body" => "GRKit needs %{model} firmware %{firmware}.",
        "software.firmware_how" => "How to Update the Firmware",
        "software.firmware_new" => "This firmware isn't supported yet",
        "software.firmware_new_body" => {
            "GRKit doesn't support firmware %{firmware} yet. Wait for a later version."
        }
        "software.app_old" => "GRKit app needs an update",
        "software.app_old_body" => "Update the app first, then install the new GRKit.",
        "app.store" => "Go to the App Store",
        "android.app.store" => "Go to Google Play",
        "cam.update_available" => "GRKit %{version} Available",
        "cam.update_available_sub" => "See Update",
        "cam.update_waiting" => "GRKit %{version} to Confirm",
        "cam.update_waiting_sub" => "Confirm on the camera · %{time} left",
        "cam.update_ready" => "GRKit %{version} to Install",
        "cam.update_ready_sub" => "Restart the camera to finish",
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
        "error.download_body" => {
            "Could not download the update. Check the internet connection and try again."
        }
        "error.service_title" => "Could Not Complete the Request",
        "loading.camera" => "Connecting to your camera…",
        "loading.update" => "Checking for updates…",
        "tab.camera" => "Camera",
        "home.title" => "Add Your Camera",
        "home.subtitle" => "Connect your camera. Manage GRKit.",
        "home.add" => "Add Camera",
        "nearby.powered_on" => "Powered on",
        "nearby.connect" => "Connect",
        "nearby.not_now" => "Not Now",
        "scan.title" => "Add a Camera",
        "scan.subtitle" => "Keep it close by, with Bluetooth on.",
        "scan.looking" => "Looking for cameras",
        "scan.nearby_off" => "Standby",
        "scan.close" => "Close",
        "scan.back" => "Back to Cameras",
        "connecting.cancel" => "Cancel",
        "connecting.title" => "Connecting to %{model}",
        "connecting.keep_on" => "Keep the camera on and nearby.",
        "connecting.bt" => "Connect over Bluetooth",
        "connecting.wifi" => "Join the camera's Wi‑Fi",
        "connecting.wifi_sub" => "If iOS asks, allow it.",
        "connecting.version" => "Read GRKit version",
        "connected.status" => "Connected",
        "connected.offline" => "Not Connected",
        "connected.reconnecting" => "Reconnecting…",
        "connected.whats_new" => "What's New",
        "connected.grkit" => "GRKit",
        "connected.firmware" => "Firmware",
        "connected.software_update" => "Software Update",
        "software.title" => "Software Update",
        "confirm.title" => "Confirm on the Camera",
        "installed.title" => "GRKit %{version} Is Installed",
        "installed.done" => "Done",
        "error.refused_title" => "The Camera Refused the Update",
        "error.lost_title" => "Connection Lost",
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
            .or_else(|| crate::strings_recipes::en(key))
            .or_else(|| crate::strings_connect::en(key))
            .unwrap_or("???"),
    }
}

fn zh(key: &str) -> Option<&'static str> {
    Some(match key {
        "update.title" => "正在更新 %{model}",
        "update.step_download" => "下载更新",
        "update.step_send" => "发送到相机",
        "update.step_confirm" => "在相机上确认",
        "update.step_restart" => "重启相机",
        "update.keep_on" => "请保持相机开机，不要离开应用。",
        "update.download_body" => "连接相机 Wi‑Fi 时，需要用蜂窝数据下载更新。请检查后重试。",
        "update.cancel" => "取消更新",
        "update.cancelling" => "正在取消…",
        "cam.more" => "更多",
        "cam.auto_connecting" => "正在查找相机…",
        "update.cancel_title" => "取消更新？",
        "update.cancel_body_send" => "已发送的部分会从相机上删除。",
        "update.cancel_body_confirm" => "相机上待确认的更新会被删除。",
        "update.cancel_body_restart" => "相机重启后不会安装这次更新。",
        "update.cancel_keep" => "继续更新",
        "update.cancel_confirm" => "取消更新",
        "update.reconnect" => "重新连接",
        "update.ok" => "好",
        "update.menu_value" => "更新 %{version}",
        "update.updated_toast" => "GRKit 已更新到 %{version}",
        "update.failed_title" => "更新没有完成",
        "update.failed_sub" => "相机开机时没有完成安装，GRKit 没有运行。",
        "update.failed_body" => {
            "请再重启一次相机。如果还是这样，把存储卡插到电脑上，在 grkit.app 重新安装。"
        }
        "update.open_site" => "打开 grkit.app",
        "restart.running_title" => "正在重启相机",
        "restart.running_body" => "请不要关闭相机或离开应用。",
        "restart.manual_body" => "请把相机关机再开机，然后点按「重新连接」。",
        "restart.auto_body" => "应用会先关闭相机再打开，然后检查版本。",
        "restart.run" => "重启相机",
        "restart.title" => "重启相机",
        "restart.subtitle" => "更新已确认，重启一次相机就会安装。",
        "confirm.subtitle" => "更新已在相机上，等你确认。",
        "confirm.step1" => "按 ADJ，移到「GRK」",
        "confirm.step2" => "选择「MOD」，再选择「%{update}」",
        "confirm.step3" => "选择「执行」",
        "confirm.waiting" => "等待确认 · 剩余 %{time}",
        "installed.verified" => "相机正在运行新版本。",
        "error.download_title" => "无法下载更新",
        "error.module_title" => "需要用存储卡更新",
        "error.module_body" => "这次更新没有相机上的模块「%{module}」。请用存储卡更新 GRKit。",
        "error.package_title" => "无法准备更新",
        "error.package_body" => "这次更新的文件不完整。请稍后再检查更新。",
        "error.card_full_title" => "存储卡空间不足",
        "error.card_full_body" => "还需要 %{size}。请在相机上删除一些照片后重试。",
        "error.not_confirmed_title" => "未在相机上确认",
        "error.not_confirmed_body" => "5 分钟内没有确认，相机已删除这次更新。",
        "error.refused_body" => "这个更新不适用于这台相机的固件。",
        "error.lost_body" => "重新连接后，应用会接着处理这次更新。",
        "software.confirm_note" => "更新需要在相机上确认，并重启一次相机。胶片风格和设置会保留。",
        "software.up_to_date" => "GRKit 已是最新",
        "software.check_failed" => "无法检查更新",
        "software.check_failed_body" => "连接相机 Wi‑Fi 时，需要用蜂窝数据下载更新。请检查后重试。",
        "software.retry" => "重试",
        "software.current" => "当前 %{version} · %{model}",
        "software.update" => "更新",
        "software.latest_sub" => "已是最新 · %{model}",
        "software.whats_new" => "新功能",
        "software.whats_new_here" => "此版本的新功能",
        "software.show_all" => "查看全部",
        "software.history" => "更新历史",
        "software.history_github" => "在 GitHub 上查看",
        "software.firmware_old" => "需要更新相机固件",
        "software.firmware_old_body" => "GRKit 需要 %{model} 固件 %{firmware}。",
        "software.firmware_how" => "了解如何更新固件",
        "software.firmware_new" => "暂不支持此固件",
        "software.firmware_new_body" => "GRKit 还不支持固件 %{firmware}，请等待后续版本。",
        "software.app_old" => "需要更新 GRKit 应用",
        "software.app_old_body" => "请先更新应用，再安装新的 GRKit。",
        "app.store" => "前往 App Store",
        "android.app.store" => "前往 Google Play",
        "cam.update_available" => "GRKit %{version} 可用",
        "cam.update_available_sub" => "查看更新",
        "cam.update_waiting" => "GRKit %{version} 待确认",
        "cam.update_waiting_sub" => "在相机上确认 · 剩余 %{time}",
        "cam.update_ready" => "GRKit %{version} 待安装",
        "cam.update_ready_sub" => "重启相机以完成更新",
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
        "error.service_title" => "无法完成请求",
        "loading.camera" => "正在连接相机…",
        "loading.update" => "正在检查更新…",
        "tab.camera" => "相机",
        "home.title" => "添加你的相机",
        "home.subtitle" => "连接相机，管理 GRKit。",
        "home.add" => "添加相机",
        "nearby.powered_on" => "已开机",
        "nearby.connect" => "连接",
        "nearby.not_now" => "暂不",
        "scan.title" => "添加相机",
        "scan.subtitle" => "请把相机放在附近，并开启相机蓝牙。",
        "scan.looking" => "正在查找相机",
        "scan.nearby_off" => "待机",
        "scan.close" => "关闭",
        "scan.back" => "返回相机列表",
        "connecting.cancel" => "取消",
        "connecting.title" => "正在连接 %{model}",
        "connecting.keep_on" => "请保持相机开机并在附近。",
        "connecting.bt" => "通过蓝牙连接",
        "connecting.wifi" => "加入相机的 Wi‑Fi",
        "connecting.wifi_sub" => "若系统询问，请允许。",
        "connecting.version" => "读取 GRKit 版本",
        "connected.status" => "已连接",
        "connected.offline" => "未连接",
        "connected.reconnecting" => "正在重新连接…",
        "connected.whats_new" => "新功能",
        "connected.grkit" => "GRKit",
        "connected.firmware" => "固件",
        "connected.software_update" => "软件更新",
        "software.title" => "软件更新",
        "confirm.title" => "在相机上确认",
        "installed.title" => "GRKit %{version} 已安装",
        "installed.done" => "完成",
        "error.refused_title" => "相机拒绝了这次更新",
        "error.lost_title" => "连接已断开",
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
            return crate::strings_looks::zh(key)
                .or_else(|| crate::strings_recipes::zh(key))
                .or_else(|| crate::strings_connect::zh(key));
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
