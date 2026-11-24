//! UI copy in the five languages of the site and the camera, chosen from the
//! system language.
//!
//! Keys match `locales/app.yml` so `nsh package` and this table stay aligned.

use nsh_core::prelude::*;

/// The language tag for a system locale: one of the five the app and the
/// camera serve. An explicit script wins over the region, so `zh-Hans-HK` stays
/// Simplified.
pub fn language_tag(locale: &str) -> &'static str {
    let tag = locale.replace('_', "-").to_ascii_lowercase();
    let mut parts = tag.split('-');
    match parts.next().unwrap_or("") {
        "zh" => {
            let subtags: Vec<&str> = parts.collect();
            if subtags.contains(&"hans") {
                "zh-Hans"
            } else if subtags
                .iter()
                .any(|part| matches!(*part, "hant" | "tw" | "hk" | "mo"))
            {
                "zh-Hant"
            } else {
                "zh-Hans"
            }
        }
        "ja" => "ja",
        "ko" => "ko",
        _ => "en",
    }
}

/// The app's current language tag, for UI copy and camera requests.
pub fn app_language() -> &'static str {
    language_tag(&locale().to_string())
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
    text_in(app_language(), key)
}

/// The text of `key` in `language`, else in English.
fn text_in(language: &str, key: &str) -> &'static str {
    let text = match language {
        "zh-Hans" => zh(key),
        "zh-Hant" => zh_hant(key),
        "ja" => ja(key),
        "ko" => ko(key),
        _ => None,
    };
    text.unwrap_or_else(|| en(key))
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
        "restart.manual" => "Restart Manually",
        "restart.title" => "Restart the Camera",
        "restart.subtitle" => "The update is confirmed. It installs when the camera restarts.",
        "confirm.subtitle" => "The update is on the camera and waits for you.",
        "confirm.step1" => "Press ADJ and move to GRK",
        "confirm.step2" => "Choose MOD, then %{update}",
        "confirm.step3" => "Choose Execute",
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
        "error.refused_body" => "This update doesn't fit the firmware of this camera.",
        "error.lost_body" => "Reconnect, and the app continues with this update.",
        "software.confirm_note" => {
            "You confirm the update on the camera, then restart it once. Your film looks and settings stay."
        }
        "software.up_to_date" => "GRKit is up to date",
        "software.check_failed" => "Couldn't check for updates",
        "software.restart_manual" => "Turn the camera off and on. It reconnects when it starts.",
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
        "cam.update_waiting_sub" => "Confirm on the camera",
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
        "error.close" => "Close",
        "tab.settings" | "settings.title" => "Settings",
        "settings.about" => "About",
        "settings.version" => "Version",
        "settings.rate" => "Rate GRKit",
        "settings.privacy" => "Privacy Policy",
        "settings.more_maker" => "More from the Maker",
        "settings.gr_snap_name" => "GR Snap",
        "settings.gr_snap_desc" => "Wireless transfer and remote for RICOH GR",
        _ => crate::strings_looks::en(key)
            .or_else(|| crate::strings_recipes::en(key))
            .or_else(|| crate::strings_connect::en(key))
            .or_else(|| crate::strings_modules::en(key))
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
        "restart.manual" => "手动重启",
        "restart.title" => "重启相机",
        "restart.subtitle" => "更新已确认，重启一次相机就会安装。",
        "confirm.subtitle" => "更新已在相机上，等你确认。",
        "confirm.step1" => "按 ADJ，移到「GRK」",
        "confirm.step2" => "选择「MOD」，再选择「%{update}」",
        "confirm.step3" => "选择「执行」",
        "installed.verified" => "相机正在运行新版本。",
        "error.download_title" => "无法下载更新",
        "error.module_title" => "需要用存储卡更新",
        "error.module_body" => "这次更新没有相机上的模块「%{module}」。请用存储卡更新 GRKit。",
        "error.package_title" => "无法准备更新",
        "error.package_body" => "这次更新的文件不完整。请稍后再检查更新。",
        "error.card_full_title" => "存储卡空间不足",
        "error.card_full_body" => "还需要 %{size}。请在相机上删除一些照片后重试。",
        "error.refused_body" => "这个更新不适用于这台相机的固件。",
        "error.lost_body" => "重新连接后，应用会接着处理这次更新。",
        "software.confirm_note" => "更新需要在相机上确认，并重启一次相机。胶片风格和设置会保留。",
        "software.up_to_date" => "GRKit 已是最新",
        "software.check_failed" => "无法检查更新",
        "software.restart_manual" => "请把相机关机再开机，开机后会自动连接。",
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
        "cam.update_waiting_sub" => "在相机上确认",
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
        "error.close" => "关闭",
        "tab.settings" | "settings.title" => "设置",
        "settings.about" => "关于",
        "settings.version" => "版本",
        "settings.rate" => "为 GRKit 评分",
        "settings.privacy" => "隐私政策",
        "settings.more_maker" => "来自同一作者",
        "settings.gr_snap_name" => "GR Snap",
        "settings.gr_snap_desc" => "RICOH GR 的无线传输与遥控",
        _ => {
            return crate::strings_looks::zh(key)
                .or_else(|| crate::strings_recipes::zh(key))
                .or_else(|| crate::strings_connect::zh(key))
                .or_else(|| crate::strings_modules::zh_hans(key));
        }
    })
}

fn zh_hant(key: &str) -> Option<&'static str> {
    Some(match key {
        "update.title" => "正在更新 %{model}",
        "update.step_download" => "下載更新",
        "update.step_send" => "傳送到相機",
        "update.step_confirm" => "在相機上確認",
        "update.step_restart" => "重新啟動相機",
        "update.keep_on" => "請保持相機開機，不要離開 App。",
        "update.download_body" => "連線相機 Wi‑Fi 時，需要用行動數據下載更新。請檢查後重試。",
        "update.cancel" => "取消更新",
        "update.cancelling" => "正在取消…",
        "cam.auto_connecting" => "正在尋找相機…",
        "update.cancel_title" => "取消更新？",
        "update.cancel_body_send" => "已傳送的部分會從相機上刪除。",
        "update.cancel_body_confirm" => "相機上待確認的更新會被刪除。",
        "update.cancel_body_restart" => "相機重新啟動後不會安裝這次更新。",
        "update.cancel_keep" => "繼續更新",
        "update.cancel_confirm" => "取消更新",
        "update.reconnect" => "重新連線",
        "update.ok" => "好",
        "update.menu_value" => "更新 %{version}",
        "update.updated_toast" => "GRKit 已更新到 %{version}",
        "update.failed_title" => "更新沒有完成",
        "update.failed_sub" => "相機開機時沒有完成安裝，GRKit 沒有執行。",
        "update.failed_body" => {
            "請再重新啟動一次相機。如果還是這樣，把記憶卡插到電腦上，在 grkit.app 重新安裝。"
        }
        "update.open_site" => "開啟 grkit.app",
        "restart.running_title" => "正在重新啟動相機",
        "restart.running_body" => "請不要關閉相機或離開 App。",
        "restart.manual_body" => "請將相機關機再開機，然後點一下「重新連線」。",
        "restart.auto_body" => "App 會先關閉相機再開啟，然後檢查版本。",
        "restart.run" => "重新啟動相機",
        "restart.manual" => "手動重新啟動",
        "restart.title" => "重新啟動相機",
        "restart.subtitle" => "更新已確認，重新啟動一次相機就會安裝。",
        "confirm.subtitle" => "更新已在相機上，等你確認。",
        "confirm.step1" => "按 ADJ，移到「GRK」",
        "confirm.step2" => "選擇「MOD」，再選擇「%{update}」",
        "confirm.step3" => "選擇「執行」",
        "installed.verified" => "相機正在執行新版本。",
        "error.download_title" => "無法下載更新",
        "error.module_title" => "需要用記憶卡更新",
        "error.module_body" => "這次更新沒有相機上的模組「%{module}」。請用記憶卡更新 GRKit。",
        "error.package_title" => "無法準備更新",
        "error.package_body" => "這次更新的檔案不完整。請稍後再檢查更新。",
        "error.card_full_title" => "記憶卡空間不足",
        "error.card_full_body" => "還需要 %{size}。請在相機上刪除一些照片後重試。",
        "error.refused_body" => "這個更新不適用於這台相機的韌體。",
        "error.lost_body" => "重新連線後，App 會接著處理這次更新。",
        "software.confirm_note" => {
            "更新需要在相機上確認，並重新啟動一次相機。底片風格和設定會保留。"
        }
        "software.up_to_date" => "GRKit 已是最新版本",
        "software.check_failed" => "無法檢查更新",
        "software.restart_manual" => "請把相機關機再開機，開機後會自動連線。",
        "software.check_failed_body" => "連線相機 Wi‑Fi 時，需要用行動數據下載更新。請檢查後重試。",
        "software.retry" => "重試",
        "software.current" => "目前 %{version} · %{model}",
        "software.update" => "更新",
        "software.latest_sub" => "已是最新 · %{model}",
        "software.whats_new" => "新功能",
        "software.whats_new_here" => "此版本的新功能",
        "software.show_all" => "查看全部",
        "software.history" => "更新紀錄",
        "software.history_github" => "在 GitHub 上查看",
        "software.firmware_old" => "需要更新相機韌體",
        "software.firmware_old_body" => "GRKit 需要 %{model} 韌體 %{firmware}。",
        "software.firmware_how" => "了解如何更新韌體",
        "software.firmware_new" => "尚不支援此韌體",
        "software.firmware_new_body" => "GRKit 還不支援韌體 %{firmware}，請等待後續版本。",
        "software.app_old" => "需要更新 GRKit App",
        "software.app_old_body" => "請先更新 App，再安裝新的 GRKit。",
        "app.store" => "前往 App Store",
        "android.app.store" => "前往 Google Play",
        "cam.update_available" => "GRKit %{version} 可用",
        "cam.update_available_sub" => "查看更新",
        "cam.update_waiting" => "GRKit %{version} 待確認",
        "cam.update_waiting_sub" => "在相機上確認",
        "cam.update_ready" => "GRKit %{version} 待安裝",
        "cam.update_ready_sub" => "重新啟動相機以完成更新",
        "android.connecting.wifi_sub" => {
            "請在 Android 網路提示中選擇相機。即使沒有網際網路，也請保持連線。"
        }
        "android.error.bt_body" => {
            "請開啟藍牙。在 Android 設定中，Android 12 及更新版本需允許 GRKit 使用「鄰近裝置」；Android 11 及更早版本需允許位置權限，並開啟定位服務。"
        }
        "android.error.wifi_body" => {
            "請在網路提示中選擇相機。Android 13 及更新版本需允許 GRKit 使用「鄰近裝置」；Android 12 及更早版本需允許位置權限，並開啟定位服務。"
        }
        "error.cancel_pending" => "取消待處理的更新",
        "software.checking" => "正在檢查更新…",
        "software.check" => "檢查更新",
        "error.download_body" => "無法下載更新。請檢查網際網路連線後重試。",
        "error.service_title" => "無法完成要求",
        "loading.camera" => "正在連線相機…",
        "loading.update" => "正在檢查更新…",
        "tab.camera" => "相機",
        "home.title" => "新增你的相機",
        "home.subtitle" => "連線相機，管理 GRKit。",
        "home.add" => "新增相機",
        "nearby.powered_on" => "已開機",
        "nearby.connect" => "連線",
        "nearby.not_now" => "暫不",
        "scan.title" => "新增相機",
        "scan.subtitle" => "請把相機放在附近，並開啟相機藍牙。",
        "scan.looking" => "正在尋找相機",
        "scan.nearby_off" => "待機",
        "scan.close" => "關閉",
        "scan.back" => "返回相機列表",
        "connecting.cancel" => "取消",
        "connecting.title" => "正在連線 %{model}",
        "connecting.keep_on" => "請保持相機開機並放在附近。",
        "connecting.bt" => "透過藍牙連線",
        "connecting.wifi" => "加入相機的 Wi‑Fi",
        "connecting.wifi_sub" => "若系統詢問，請允許。",
        "connecting.version" => "讀取 GRKit 版本",
        "connected.status" => "已連線",
        "connected.offline" => "未連線",
        "connected.reconnecting" => "正在重新連線…",
        "connected.whats_new" => "新功能",
        "connected.grkit" => "GRKit",
        "connected.firmware" => "韌體",
        "connected.software_update" => "軟體更新",
        "software.title" => "軟體更新",
        "confirm.title" => "在相機上確認",
        "installed.title" => "GRKit %{version} 已安裝",
        "installed.done" => "完成",
        "error.refused_title" => "相機拒絕了這次更新",
        "error.lost_title" => "連線已中斷",
        "error.card_ready_title" => "相機尚未就緒",
        "error.card_ready_body" => "記憶卡仍在啟動，請稍後再試。",
        "error.bt_title" => "無法使用藍牙",
        "error.bt_body" => "請開啟藍牙並允許 GRKit 使用。",
        "error.try_again" => "重試",
        "error.close" => "關閉",
        "tab.settings" | "settings.title" => "設定",
        "settings.about" => "關於",
        "settings.version" => "版本",
        "settings.rate" => "為 GRKit 評分",
        "settings.privacy" => "隱私權政策",
        "settings.more_maker" => "來自同一作者",
        "settings.gr_snap_name" => "GR Snap",
        "settings.gr_snap_desc" => "RICOH GR 的無線傳輸與遙控",
        _ => {
            return crate::strings_looks::zh_hant(key)
                .or_else(|| crate::strings_recipes::zh_hant(key))
                .or_else(|| crate::strings_connect::zh_hant(key))
                .or_else(|| crate::strings_modules::zh_hant(key));
        }
    })
}

fn ja(key: &str) -> Option<&'static str> {
    Some(match key {
        "update.title" => "%{model} をアップデート中",
        "update.step_download" => "アップデートをダウンロード",
        "update.step_send" => "カメラに送信",
        "update.step_confirm" => "カメラで確認",
        "update.step_restart" => "カメラを再起動",
        "update.keep_on" => "カメラの電源を入れたまま、アプリを閉じないでください。",
        "update.download_body" => {
            "カメラの Wi‑Fi に接続中は、モバイルデータ通信でアップデートをダウンロードします。確認してもう一度お試しください。"
        }
        "update.cancel" => "アップデートをキャンセル",
        "update.cancelling" => "キャンセル中…",
        "cam.auto_connecting" => "カメラを探しています…",
        "update.cancel_title" => "アップデートをキャンセルしますか？",
        "update.cancel_body_send" => "送信済みのデータはカメラから削除されます。",
        "update.cancel_body_confirm" => "カメラで確認待ちのアップデートは削除されます。",
        "update.cancel_body_restart" => {
            "カメラを再起動しても、このアップデートはインストールされません。"
        }
        "update.cancel_keep" => "アップデートを続ける",
        "update.cancel_confirm" => "アップデートをキャンセル",
        "update.reconnect" => "再接続",
        "update.ok" => "OK",
        "update.menu_value" => "アップデート %{version}",
        "update.updated_toast" => "GRKit を %{version} にアップデートしました",
        "update.failed_title" => "アップデートが完了しませんでした",
        "update.failed_sub" => {
            "インストールが完了しないままカメラが起動したため、GRKit は動作していません。"
        }
        "update.failed_body" => {
            "カメラをもう一度再起動してください。それでも GRKit が起動しない場合は、メモリーカードをコンピューターに挿入し、grkit.app で GRKit をインストールし直してください。"
        }
        "update.open_site" => "grkit.app を開く",
        "restart.running_title" => "カメラを再起動中",
        "restart.running_body" => "カメラの電源を切ったり、アプリを閉じたりしないでください。",
        "restart.manual_body" => "カメラの電源を入れ直してから、「再接続」をタップしてください。",
        "restart.auto_body" => "アプリがカメラの電源を入れ直し、バージョンを確認します。",
        "restart.run" => "カメラを再起動",
        "restart.manual" => "手動で再起動",
        "restart.title" => "カメラを再起動",
        "restart.subtitle" => {
            "アップデートを確認しました。カメラを再起動するとインストールされます。"
        }
        "confirm.subtitle" => "アップデートはカメラに届いています。確認してください。",
        "confirm.step1" => "ADJ を押して「GRK」に移動",
        "confirm.step2" => "「MOD」を選び、「%{update}」を選択",
        "confirm.step3" => "「実行」を選択",
        "installed.verified" => "カメラは新しいバージョンで動作しています。",
        "error.download_title" => "アップデートをダウンロードできません",
        "error.module_title" => "メモリーカードでアップデート",
        "error.module_body" => {
            "このアップデートには、カメラにあるモジュール「%{module}」が含まれていません。メモリーカードで GRKit をアップデートしてください。"
        }
        "error.package_title" => "アップデートを準備できません",
        "error.package_body" => {
            "このアップデートのファイルが不完全です。しばらくしてからもう一度確認してください。"
        }
        "error.card_full_title" => "カードの空き容量不足",
        "error.card_full_body" => {
            "あと %{size} 必要です。カメラで写真を削除してから、もう一度お試しください。"
        }
        "error.refused_body" => {
            "このアップデートは、このカメラのファームウェアに対応していません。"
        }
        "error.lost_body" => "再接続すると、アプリがこのアップデートを続行します。",
        "software.confirm_note" => {
            "アップデートはカメラで確認し、カメラを一度再起動します。フィルムルックと設定はそのまま残ります。"
        }
        "software.up_to_date" => "GRKit は最新です",
        "software.check_failed" => "アップデートを確認できません",
        "software.restart_manual" => {
            "カメラの電源を入れ直してください。起動すると自動で接続します。"
        }
        "software.check_failed_body" => {
            "カメラの Wi‑Fi に接続中は、モバイルデータ通信でアップデートをダウンロードします。確認してもう一度お試しください。"
        }
        "software.retry" => "再試行",
        "software.current" => "現在 %{version} · %{model}",
        "software.update" => "アップデート",
        "software.latest_sub" => "最新 · %{model}",
        "software.whats_new" => "新機能",
        "software.whats_new_here" => "このバージョンの新機能",
        "software.show_all" => "すべて表示",
        "software.history" => "アップデート履歴",
        "software.history_github" => "GitHub で見る",
        "software.firmware_old" => "ファームウェアのアップデートが必要",
        "software.firmware_old_body" => {
            "GRKit には %{model} のファームウェア %{firmware} が必要です。"
        }
        "software.firmware_how" => "ファームウェアのアップデート方法",
        "software.firmware_new" => "このファームウェアにはまだ対応していません",
        "software.firmware_new_body" => {
            "GRKit はファームウェア %{firmware} にまだ対応していません。今後のバージョンをお待ちください。"
        }
        "software.app_old" => "GRKit アプリのアップデートが必要",
        "software.app_old_body" => {
            "先にアプリをアップデートしてから、新しい GRKit をインストールしてください。"
        }
        "app.store" => "App Store へ",
        "android.app.store" => "Google Play へ",
        "cam.update_available" => "GRKit %{version} が利用可能",
        "cam.update_available_sub" => "アップデートを見る",
        "cam.update_waiting" => "GRKit %{version} 確認待ち",
        "cam.update_waiting_sub" => "カメラで確認",
        "cam.update_ready" => "GRKit %{version} インストール待ち",
        "cam.update_ready_sub" => "カメラを再起動して完了",
        "android.connecting.wifi_sub" => {
            "Android のネットワーク確認でカメラを選択してください。インターネットに接続できなくても、接続を維持してください。"
        }
        "android.error.bt_body" => {
            "Bluetooth をオンにしてください。Android の設定で、Android 12 以降は GRKit に「付近のデバイス」を許可します。Android 11 以前は「位置情報」を許可し、位置情報サービスをオンにします。"
        }
        "android.error.wifi_body" => {
            "ネットワーク確認でカメラを選択してください。Android 13 以降は GRKit に「付近のデバイス」を許可します。Android 12 以前は「位置情報」を許可し、位置情報サービスをオンにします。"
        }
        "error.cancel_pending" => "保留中のアップデートをキャンセル",
        "software.checking" => "アップデートを確認中…",
        "software.check" => "アップデートを確認",
        "error.download_body" => {
            "アップデートをダウンロードできませんでした。インターネット接続を確認して、もう一度お試しください。"
        }
        "error.service_title" => "リクエストを完了できません",
        "loading.camera" => "カメラに接続中…",
        "loading.update" => "アップデートを確認中…",
        "tab.camera" => "カメラ",
        "home.title" => "カメラを追加",
        "home.subtitle" => "カメラに接続して、GRKit を管理。",
        "home.add" => "カメラを追加",
        "nearby.powered_on" => "電源オン",
        "nearby.connect" => "接続",
        "nearby.not_now" => "後で",
        "scan.title" => "カメラを追加",
        "scan.subtitle" => "カメラを近くに置き、Bluetooth をオンにしてください。",
        "scan.looking" => "カメラを探しています",
        "scan.nearby_off" => "スタンバイ",
        "scan.close" => "閉じる",
        "scan.back" => "カメラ一覧に戻る",
        "connecting.cancel" => "キャンセル",
        "connecting.title" => "%{model} に接続中",
        "connecting.keep_on" => "カメラの電源を入れたまま、近くに置いてください。",
        "connecting.bt" => "Bluetooth で接続",
        "connecting.wifi" => "カメラの Wi‑Fi に接続",
        "connecting.wifi_sub" => "確認が表示されたら許可してください。",
        "connecting.version" => "GRKit のバージョンを読み取り",
        "connected.status" => "接続済み",
        "connected.offline" => "未接続",
        "connected.reconnecting" => "再接続中…",
        "connected.whats_new" => "新機能",
        "connected.grkit" => "GRKit",
        "connected.firmware" => "ファームウェア",
        "connected.software_update" => "ソフトウェアアップデート",
        "software.title" => "ソフトウェアアップデート",
        "confirm.title" => "カメラで確認",
        "installed.title" => "GRKit %{version} をインストールしました",
        "installed.done" => "完了",
        "error.refused_title" => "カメラがアップデートを拒否しました",
        "error.lost_title" => "接続が切れました",
        "error.card_ready_title" => "カメラの準備ができていません",
        "error.card_ready_body" => {
            "メモリーカードの準備中です。少し待ってからもう一度お試しください。"
        }
        "error.bt_title" => "Bluetooth を使用できません",
        "error.bt_body" => "Bluetooth をオンにして、GRKit に許可してください。",
        "error.try_again" => "再試行",
        "error.close" => "閉じる",
        "tab.settings" | "settings.title" => "設定",
        "settings.about" => "このアプリについて",
        "settings.version" => "バージョン",
        "settings.rate" => "GRKit を評価",
        "settings.privacy" => "プライバシーポリシー",
        "settings.more_maker" => "同じ開発者のアプリ",
        "settings.gr_snap_name" => "GR Snap",
        "settings.gr_snap_desc" => "RICOH GR のワイヤレス転送とリモート撮影",
        _ => {
            return crate::strings_looks::ja(key)
                .or_else(|| crate::strings_recipes::ja(key))
                .or_else(|| crate::strings_connect::ja(key))
                .or_else(|| crate::strings_modules::ja(key));
        }
    })
}

fn ko(key: &str) -> Option<&'static str> {
    Some(match key {
        "update.title" => "%{model} 업데이트 중",
        "update.step_download" => "업데이트 다운로드",
        "update.step_send" => "카메라로 전송",
        "update.step_confirm" => "카메라에서 확인",
        "update.step_restart" => "카메라 재시작",
        "update.keep_on" => "카메라를 켜 두고 앱을 벗어나지 마세요.",
        "update.download_body" => {
            "카메라 Wi‑Fi에 연결된 동안에는 모바일 데이터로 업데이트를 다운로드합니다. 확인한 후 다시 시도하세요."
        }
        "update.cancel" => "업데이트 취소",
        "update.cancelling" => "취소하는 중…",
        "cam.auto_connecting" => "카메라를 찾는 중…",
        "update.cancel_title" => "업데이트를 취소할까요?",
        "update.cancel_body_send" => "전송된 데이터는 카메라에서 삭제됩니다.",
        "update.cancel_body_confirm" => "카메라에서 확인을 기다리는 업데이트가 삭제됩니다.",
        "update.cancel_body_restart" => "카메라를 재시작해도 이 업데이트는 설치되지 않습니다.",
        "update.cancel_keep" => "업데이트 계속",
        "update.cancel_confirm" => "업데이트 취소",
        "update.reconnect" => "다시 연결",
        "update.ok" => "확인",
        "update.menu_value" => "업데이트 %{version}",
        "update.updated_toast" => "GRKit을 %{version}(으)로 업데이트했습니다",
        "update.failed_title" => "업데이트가 완료되지 않음",
        "update.failed_sub" => "설치가 끝나지 않은 채 카메라가 시작되어 GRKit이 실행되지 않습니다.",
        "update.failed_body" => {
            "카메라를 한 번 더 재시작하세요. 그래도 GRKit이 시작되지 않으면 메모리 카드를 컴퓨터에 넣고 grkit.app에서 GRKit을 다시 설치하세요."
        }
        "update.open_site" => "grkit.app 열기",
        "restart.running_title" => "카메라 재시작 중",
        "restart.running_body" => "카메라를 끄거나 앱을 벗어나지 마세요.",
        "restart.manual_body" => "카메라를 껐다 켠 후 「다시 연결」을 탭하세요.",
        "restart.auto_body" => "앱이 카메라를 껐다 켠 후 버전을 확인합니다.",
        "restart.run" => "카메라 재시작",
        "restart.manual" => "직접 재시작",
        "restart.title" => "카메라 재시작",
        "restart.subtitle" => "업데이트가 확인되었습니다. 카메라를 재시작하면 설치됩니다.",
        "confirm.subtitle" => "업데이트가 카메라에 있습니다. 확인해 주세요.",
        "confirm.step1" => "ADJ를 누르고 「GRK」로 이동",
        "confirm.step2" => "「MOD」를 선택한 뒤 「%{update}」 선택",
        "confirm.step3" => "「실행」 선택",
        "installed.verified" => "카메라가 새 버전으로 실행 중입니다.",
        "error.download_title" => "업데이트를 다운로드할 수 없음",
        "error.module_title" => "메모리 카드로 업데이트",
        "error.module_body" => {
            "이 업데이트에는 카메라에 있는 모듈 「%{module}」이(가) 없습니다. 메모리 카드로 GRKit을 업데이트하세요."
        }
        "error.package_title" => "업데이트를 준비할 수 없음",
        "error.package_body" => {
            "이 업데이트의 파일이 불완전합니다. 나중에 업데이트를 다시 확인하세요."
        }
        "error.card_full_title" => "카드 공간 부족",
        "error.card_full_body" => {
            "%{size}이(가) 더 필요합니다. 카메라에서 사진을 삭제한 후 다시 시도하세요."
        }
        "error.refused_body" => "이 업데이트는 이 카메라의 펌웨어에 맞지 않습니다.",
        "error.lost_body" => "다시 연결하면 앱이 이 업데이트를 이어서 진행합니다.",
        "software.confirm_note" => {
            "업데이트는 카메라에서 확인한 뒤 카메라를 한 번 재시작합니다. 필름 룩과 설정은 유지됩니다."
        }
        "software.up_to_date" => "GRKit이 최신 버전입니다",
        "software.check_failed" => "업데이트를 확인할 수 없음",
        "software.restart_manual" => "카메라를 껐다 켜 주세요. 켜지면 자동으로 연결됩니다.",
        "software.check_failed_body" => {
            "카메라 Wi‑Fi에 연결된 동안에는 모바일 데이터로 업데이트를 다운로드합니다. 확인한 후 다시 시도하세요."
        }
        "software.retry" => "다시 시도",
        "software.current" => "현재 %{version} · %{model}",
        "software.update" => "업데이트",
        "software.latest_sub" => "최신 · %{model}",
        "software.whats_new" => "새로운 기능",
        "software.whats_new_here" => "이 버전의 새로운 기능",
        "software.show_all" => "모두 보기",
        "software.history" => "업데이트 기록",
        "software.history_github" => "GitHub에서 보기",
        "software.firmware_old" => "카메라 펌웨어 업데이트 필요",
        "software.firmware_old_body" => {
            "GRKit을 쓰려면 %{model} 펌웨어 %{firmware}이(가) 필요합니다."
        }
        "software.firmware_how" => "펌웨어 업데이트 방법",
        "software.firmware_new" => "아직 지원하지 않는 펌웨어",
        "software.firmware_new_body" => {
            "GRKit은 아직 펌웨어 %{firmware}을(를) 지원하지 않습니다. 이후 버전을 기다려 주세요."
        }
        "software.app_old" => "GRKit 앱 업데이트 필요",
        "software.app_old_body" => "먼저 앱을 업데이트한 후 새 GRKit을 설치하세요.",
        "app.store" => "App Store로 이동",
        "android.app.store" => "Google Play로 이동",
        "cam.update_available" => "GRKit %{version} 사용 가능",
        "cam.update_available_sub" => "업데이트 보기",
        "cam.update_waiting" => "GRKit %{version} 확인 대기",
        "cam.update_waiting_sub" => "카메라에서 확인",
        "cam.update_ready" => "GRKit %{version} 설치 대기",
        "cam.update_ready_sub" => "카메라를 재시작해 완료",
        "android.connecting.wifi_sub" => {
            "Android 네트워크 알림에서 카메라를 선택하세요. 인터넷이 없어도 연결을 유지하세요."
        }
        "android.error.bt_body" => {
            "Bluetooth를 켜세요. Android 설정에서 Android 12 이상은 GRKit에 근처 기기 권한을 허용하세요. Android 11 이하는 위치 권한을 허용하고 위치 서비스를 켜세요."
        }
        "android.error.wifi_body" => {
            "네트워크 알림에서 카메라를 선택하세요. Android 13 이상은 GRKit에 근처 기기 권한을 허용하세요. Android 12 이하는 위치 권한을 허용하고 위치 서비스를 켜세요."
        }
        "error.cancel_pending" => "대기 중인 업데이트 취소",
        "software.checking" => "업데이트 확인 중…",
        "software.check" => "업데이트 확인",
        "error.download_body" => {
            "업데이트를 다운로드하지 못했습니다. 인터넷 연결을 확인하고 다시 시도하세요."
        }
        "error.service_title" => "요청을 완료할 수 없음",
        "loading.camera" => "카메라에 연결하는 중…",
        "loading.update" => "업데이트 확인 중…",
        "tab.camera" => "카메라",
        "home.title" => "카메라 추가",
        "home.subtitle" => "카메라를 연결하고 GRKit을 관리하세요.",
        "home.add" => "카메라 추가",
        "nearby.powered_on" => "켜짐",
        "nearby.connect" => "연결",
        "nearby.not_now" => "나중에",
        "scan.title" => "카메라 추가",
        "scan.subtitle" => "카메라를 가까이 두고 Bluetooth를 켜 두세요.",
        "scan.looking" => "카메라를 찾는 중",
        "scan.nearby_off" => "대기",
        "scan.close" => "닫기",
        "scan.back" => "카메라 목록으로",
        "connecting.cancel" => "취소",
        "connecting.title" => "%{model}에 연결하는 중",
        "connecting.keep_on" => "카메라를 켜 두고 가까이 두세요.",
        "connecting.bt" => "Bluetooth로 연결",
        "connecting.wifi" => "카메라 Wi‑Fi에 연결",
        "connecting.wifi_sub" => "요청이 표시되면 허용하세요.",
        "connecting.version" => "GRKit 버전 읽기",
        "connected.status" => "연결됨",
        "connected.offline" => "연결 안 됨",
        "connected.reconnecting" => "다시 연결하는 중…",
        "connected.whats_new" => "새로운 기능",
        "connected.grkit" => "GRKit",
        "connected.firmware" => "펌웨어",
        "connected.software_update" => "소프트웨어 업데이트",
        "software.title" => "소프트웨어 업데이트",
        "confirm.title" => "카메라에서 확인",
        "installed.title" => "GRKit %{version} 설치됨",
        "installed.done" => "완료",
        "error.refused_title" => "카메라가 업데이트를 거부함",
        "error.lost_title" => "연결이 끊김",
        "error.card_ready_title" => "카메라가 준비되지 않음",
        "error.card_ready_body" => "메모리 카드를 준비하는 중입니다. 잠시 후 다시 시도하세요.",
        "error.bt_title" => "Bluetooth를 사용할 수 없음",
        "error.bt_body" => "Bluetooth를 켜고 GRKit에 권한을 허용하세요.",
        "error.try_again" => "다시 시도",
        "error.close" => "닫기",
        "tab.settings" | "settings.title" => "설정",
        "settings.about" => "정보",
        "settings.version" => "버전",
        "settings.rate" => "GRKit 평가하기",
        "settings.privacy" => "개인정보 처리방침",
        "settings.more_maker" => "같은 개발자의 앱",
        "settings.gr_snap_name" => "GR Snap",
        "settings.gr_snap_desc" => "RICOH GR 무선 전송과 원격 촬영",
        _ => {
            return crate::strings_looks::ko(key)
                .or_else(|| crate::strings_recipes::ko(key))
                .or_else(|| crate::strings_connect::ko(key))
                .or_else(|| crate::strings_modules::ko(key));
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

#[cfg(test)]
mod tests {
    use super::{en, language_tag, text_in};
    use std::collections::{BTreeMap, BTreeSet};

    const LOCALES: &str = include_str!("../locales/app.yml");
    /// The tags of `locales/app.yml`, then the tags of the lookup.
    const LANGUAGES: [(&str, &str); 5] = [
        ("en", "en"),
        ("zh-CN", "zh-Hans"),
        ("zh-TW", "zh-Hant"),
        ("ja", "ja"),
        ("ko", "ko"),
    ];
    /// Keys in the file that no screen reads.
    const FILE_ONLY: [&str; 2] = ["nearby.title_suffix", "recipes.read_only_note"];
    /// No Simplified Chinese text yet: this key shows in English there.
    const NO_ZH_HANS: &str = "error.download_body";
    /// The Simplified Chinese text counts the total, the English the rest.
    const OWN_ZH_HANS_PLACEHOLDER: &str = "looks.use_more";

    fn entries() -> BTreeMap<&'static str, BTreeMap<&'static str, &'static str>> {
        let mut entries: BTreeMap<_, BTreeMap<_, _>> = BTreeMap::new();
        let mut key = "";
        for line in LOCALES.lines().skip(1) {
            if let Some(field) = line.strip_prefix("  ") {
                let (language, text) = field.split_once(':').expect("a language line");
                entries.entry(key).or_default().insert(language, text);
            } else if let Some(name) = line.strip_suffix(':') {
                key = name;
            }
        }
        entries
    }

    fn placeholders(text: &str) -> BTreeSet<&str> {
        text.split("%{")
            .skip(1)
            .filter_map(|part| part.split_once('}').map(|(name, _)| name))
            .collect()
    }

    #[test]
    fn every_key_has_five_languages_and_the_english_placeholders() {
        let entries = entries();
        assert!(entries.len() > 600);
        for (key, texts) in &entries {
            let english = placeholders(texts.get("en").expect(key));
            for (file_tag, language) in LANGUAGES {
                if key == &NO_ZH_HANS && language == "zh-Hans" {
                    assert_eq!(text_in(language, key), en(key));
                    continue;
                }
                let text = texts
                    .get(file_tag)
                    .unwrap_or_else(|| panic!("{key}: {file_tag}"));
                if key == &OWN_ZH_HANS_PLACEHOLDER && language == "zh-Hans" {
                    continue;
                }
                assert_eq!(placeholders(text), english, "{key}: {file_tag}");
                if !FILE_ONLY.contains(key) {
                    let shown = text_in(language, key);
                    assert_ne!(shown, "???", "{key}: {language}");
                    assert_eq!(placeholders(shown), english, "{key}: {language}");
                }
            }
        }
    }

    #[test]
    fn system_locales_map_to_the_five_tags() {
        for (locale, tag) in [
            ("en-US", "en"),
            ("fr-FR", "en"),
            ("zh-Hans-CN", "zh-Hans"),
            ("zh-CN", "zh-Hans"),
            ("zh-Hans-HK", "zh-Hans"),
            ("zh-Hant-HK", "zh-Hant"),
            ("zh_TW", "zh-Hant"),
            ("zh-HK", "zh-Hant"),
            ("zh-MO", "zh-Hant"),
            ("ja-JP", "ja"),
            ("ko-KR", "ko"),
        ] {
            assert_eq!(language_tag(locale), tag, "{locale}");
        }
    }
}
