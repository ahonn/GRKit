//! Connection error copy in English and Simplified Chinese. Keys match `locales/app.yml`.

pub fn en(key: &str) -> Option<&'static str> {
    Some(match key {
        "conn.app_old.title" => "GRKit App Needs an Update",
        "conn.app_old.body" => {
            "The GRKit on the camera is newer than this app. Update the app, then connect again."
        }
        "conn.unsupported.body" => {
            "The camera replied, but GRKit couldn't understand the response. Try again."
        }
        "conn.open_settings" => "Open Settings",
        "conn.switched_wifi" => "I've Switched Wi‑Fi",
        "conn.ble_failed.title" => "Bluetooth Connection Failed",
        "conn.ble_failed.body" => "Toggle Bluetooth off and on, then retry.",
        "conn.ble_auth.title" => "Pairing Didn't Complete",
        "conn.ble_auth.body" => {
            "The camera didn't finish pairing. On the camera, select Pairing → Execute Pairing, then try again. If that fails, forget this camera in Settings → Bluetooth first."
        }
        "conn.ble_pair.title" => "Pair on the Camera",
        "conn.ble_pair.body" => {
            "On the camera, select Pairing → Execute Pairing, then tap Try Again."
        }
        "conn.ble_stale.title" => "Camera Forgot This Device",
        "conn.ble_stale.body" => {
            "In Settings → Bluetooth, tap ⓘ next to this camera and choose Forget This Device. Then pair again on the camera and try again."
        }
        "conn.ble_timeout.title" => "Camera Not Responding",
        "conn.ble_timeout.body" => "Turn the camera on and keep it close by, then try again.",
        "conn.wifi_denied.title" => "Wi‑Fi Join Canceled",
        "conn.wifi_denied.body" => {
            "Tap Join on the system prompt, or join the camera's Wi‑Fi in Settings › Wi‑Fi."
        }
        "conn.wifi_password.title" => "Wrong Wi‑Fi Password",
        "conn.wifi_password.body" => {
            "The camera's Wi‑Fi password may have changed. Pair on the camera again, then try again."
        }
        "conn.wifi_ssid.title" => "Invalid Wi‑Fi Name",
        "conn.wifi_ssid.body" => "Check the camera's SSID.",
        "conn.wifi_config.title" => "Wi‑Fi Settings Rejected",
        "conn.wifi_config.body" => "iOS couldn't apply these Wi‑Fi settings. Try again.",
        "android.conn.wifi_config.body" => {
            "Android couldn't apply these Wi‑Fi settings. Try again."
        }
        "conn.wifi_failed.title" => "Camera Wi‑Fi Not Ready",
        "conn.wifi_failed.body" => {
            "The camera's Wi‑Fi hasn't started yet. Keep the camera on for a few seconds, then try again."
        }
        "conn.wifi_timeout.title" => "Camera Wi‑Fi Unreachable",
        "conn.wifi_timeout.body" => "Make sure the camera is on and its Wi‑Fi is enabled.",
        "conn.network_not_ready.title" => "Still Switching to Camera Wi‑Fi",
        "conn.network_not_ready.body" => {
            "Your device joined the camera's Wi‑Fi but hasn't switched over yet. Wait a moment, then try again."
        }
        "conn.subnet.title" => "Wi‑Fi Address Conflict",
        "conn.subnet.body" => {
            "Your phone reached your router, not the camera: both use 192.168.0.x. Turn on Airplane Mode, turn Wi‑Fi back on, then join the camera's Wi‑Fi."
        }
        "conn.verify.title" => "Camera Isn't Responding",
        "conn.verify.body" => "Wi‑Fi joined but no reply. Restart the camera.",
        "conn.local_network.title" => "Local Network Access Denied",
        "conn.local_network.body" => {
            "GRKit needs Local Network access to talk to your camera. Enable it in Settings → GRKit → Local Network."
        }
        "android.conn.local_network.body" => {
            "GRKit needs the Nearby devices permission to talk to your camera over Wi‑Fi. Tap Open Settings, then allow it in Permissions → Nearby devices."
        }
        "conn.not_gr.title" => "Not a GR Camera",
        "conn.not_gr.body" => {
            "Looks like you're on a different Wi‑Fi network. Switch to your camera's Wi‑Fi."
        }
        "conn.unsupported.title" => "Unsupported Camera Response",
        "conn.unknown.title" => "Couldn't Connect",
        "conn.unknown.body" => "Something went wrong. Please retry.",
        "cam.title" => "Camera",
        "cam.detect" => "Joined camera Wi‑Fi? Detect now",
        "cam.nearby" => "On · Nearby",
        "cam.others" => "Other Cameras",
        "cam.lost_title" => "Lost Connection",
        "cam.current" => "On the Camera",
        "cam.current_none" => "No recipe or Film Look in use",
        "cam.remove" => "Remove Camera",
        "cam.remove_body" => "Remove “%{name}” from saved cameras?",
        "cam.remove_confirm" => "Remove",
        "cam.cancel" => "Cancel",
        "cam.connected" => "Connected",
        "cam.saved" => "Saved Cameras",
        "cam.saved_note" => "Only one camera can connect at a time.",
        "cam.settings_group" => "Camera",
        "cam.help" => "Help & Feedback",
        "cam.disconnect" => "Disconnect",
        "cam.disconnect_title" => "Disconnect Camera",
        "cam.disconnect_body" => {
            "The camera's Wi‑Fi turns off, and your phone returns to its previous network."
        }
        "cam.disconnect_confirm" => "Disconnect",
        "cam.power" => "Turn Off Camera",
        "cam.power_title" => "Turn Off Camera",
        "cam.power_body" => "The camera turns off.",
        "cam.power_confirm" => "Turn Off",
        "cam.prompt_title" => "Connect %{model}?",
        "cam.prompt_body" => "The camera is on and nearby.",
        "cam.first_time" => "First connection? On the camera, choose",
        "cam.first_time_device" => "First time with this camera? On the camera, choose",
        "cam.path_common" => "Pairing → Execute Pairing",
        "cam.path_gr3" => "Bluetooth Setting → Pairing → Execute Pairing",
        "cam.path_gr4" => "Wireless Communication → Pairing → Execute Pairing",
        "cam.step_pair" => "Pairing with camera",
        "cam.step_wake" => "Connecting to camera",
        "cam.step_wake_sub" => "Waking the camera over Bluetooth.",
        "cam.step_pair_gr3" => "The camera may show a passkey. Enter it when iOS asks.",
        "cam.step_pair_gr4" => "The camera may ask to confirm pairing. Accept it on the camera.",
        "cam.step_wifi" => "Joining camera Wi‑Fi",
        "cam.step_wifi_sub" => "If iOS asks, allow GRKit to connect to the camera.",
        "cam.step_wifi_slow" => "GR IV takes longer to start its Wi‑Fi. Please wait.",
        "cam.step_verify" => "Contacting the camera",
        "cam.step_verify_sub" => "Checking the connection.",
        "cam.no_grkit_title" => "No GRKit on the Camera",
        "cam.no_grkit_body" => "Install GRKit 1.7.0 or later on the memory card.",
        "cam.install_guide" => "Installation Guide",
        "android.cam.step_wifi_sub" => "If Android asks, allow GRKit to connect to the camera.",
        "android.cam.step_pair_gr3" => "The camera may show a passkey. Enter it when Android asks.",
        _ => return None,
    })
}

pub fn zh(key: &str) -> Option<&'static str> {
    Some(match key {
        "conn.app_old.title" => "需要更新 GRKit 应用",
        "conn.app_old.body" => "相机上的 GRKit 比这个应用新。请先更新应用，再连接。",
        "conn.unsupported.body" => "相机有响应，但 GRKit 无法识别。请重试。",
        "conn.open_settings" => "打开设置",
        "conn.switched_wifi" => "已切换 Wi‑Fi",
        "conn.ble_failed.title" => "蓝牙连接失败",
        "conn.ble_failed.body" => "请关闭蓝牙后再打开，然后重试。",
        "conn.ble_auth.title" => "配对未完成",
        "conn.ble_auth.body" => {
            "相机没有完成配对。请在相机上选择「配对」→「执行配对」，然后重试。如果仍然失败，请先在「设置」→「蓝牙」中忽略此相机。"
        }
        "conn.ble_pair.title" => "请在相机上执行蓝牙配对",
        "conn.ble_pair.body" => "请在相机上选择「配对」→「执行配对」，然后点按「重试」。",
        "conn.ble_stale.title" => "相机已不记得此设备",
        "conn.ble_stale.body" => {
            "请在「设置」→「蓝牙」中点击此相机旁的 ⓘ，选择「忽略此设备」。然后在相机上重新配对并重试。"
        }
        "conn.ble_timeout.title" => "相机无响应",
        "conn.ble_timeout.body" => "请打开相机电源并放在附近，然后重试。",
        "conn.wifi_denied.title" => "已取消加入 Wi‑Fi",
        "conn.wifi_denied.body" => {
            "请在系统提示中点「加入」，或在「设置」→「Wi‑Fi」中加入相机 Wi‑Fi。"
        }
        "conn.wifi_password.title" => "Wi‑Fi 密码错误",
        "conn.wifi_password.body" => "相机的 Wi‑Fi 密码可能已更改。请在相机上重新配对，然后重试。",
        "conn.wifi_ssid.title" => "Wi‑Fi 名称无效",
        "conn.wifi_ssid.body" => "请检查相机的 SSID。",
        "conn.wifi_config.title" => "Wi‑Fi 设置被拒绝",
        "conn.wifi_config.body" => "iOS 无法应用此 Wi‑Fi 设置，请重试。",
        "android.conn.wifi_config.body" => "Android 无法应用此 Wi‑Fi 设置，请重试。",
        "conn.wifi_failed.title" => "相机 Wi‑Fi 尚未就绪",
        "conn.wifi_failed.body" => "相机的 Wi‑Fi 还没有启动。请保持相机开机几秒钟，然后重试。",
        "conn.wifi_timeout.title" => "无法连接相机 Wi‑Fi",
        "conn.wifi_timeout.body" => "请确认相机已开机且 Wi‑Fi 已开启。",
        "conn.network_not_ready.title" => "正在切换到相机 Wi‑Fi",
        "conn.network_not_ready.body" => {
            "设备已加入相机 Wi‑Fi，但还没有切换过去。请稍等片刻后重试。"
        }
        "conn.subnet.title" => "Wi‑Fi 网段与相机冲突",
        "conn.subnet.body" => {
            "手机连到了路由器，不是相机（两者都用 192.168.0.x）。请打开飞行模式，只开 Wi‑Fi，再加入相机 Wi‑Fi。"
        }
        "conn.verify.title" => "相机无响应",
        "conn.verify.body" => "已加入 Wi‑Fi 但相机无响应，请重启相机。",
        "conn.local_network.title" => "本地网络访问被拒绝",
        "conn.local_network.body" => {
            "GRKit 需要本地网络权限才能与相机通信。请前往「设置」→「GRKit」→「本地网络」开启。"
        }
        "android.conn.local_network.body" => {
            "GRKit 需要“附近的设备”权限才能通过 Wi‑Fi 与相机通信。请轻点“打开设置”，然后在“权限”→“附近的设备”中允许。"
        }
        "conn.not_gr.title" => "不是 GR 相机",
        "conn.not_gr.body" => "看起来你连接到了其他 Wi‑Fi，请切换到相机的 Wi‑Fi。",
        "conn.unsupported.title" => "无法识别的相机响应",
        "conn.unknown.title" => "连接失败",
        "conn.unknown.body" => "出了点问题，请重试。",
        "cam.title" => "相机",
        "cam.detect" => "已加入相机 Wi‑Fi？立即检测",
        "cam.nearby" => "已开机 · 在附近",
        "cam.others" => "其他相机",
        "cam.lost_title" => "连接已断开",
        "cam.current" => "相机当前",
        "cam.current_none" => "未使用配方或胶片风格",
        "cam.remove" => "移除相机",
        "cam.remove_body" => "确定从已保存的相机中移除「%{name}」吗？",
        "cam.remove_confirm" => "移除",
        "cam.cancel" => "取消",
        "cam.connected" => "已连接",
        "cam.saved" => "已保存的相机",
        "cam.saved_note" => "一次只能连接一台相机。",
        "cam.settings_group" => "相机",
        "cam.help" => "帮助与反馈",
        "cam.disconnect" => "断开连接",
        "cam.disconnect_title" => "断开相机",
        "cam.disconnect_body" => "相机 Wi‑Fi 会关闭，手机连回原来的网络。",
        "cam.disconnect_confirm" => "断开",
        "cam.power" => "关闭相机",
        "cam.power_title" => "关闭相机",
        "cam.power_body" => "相机会关机。",
        "cam.power_confirm" => "关机",
        "cam.prompt_title" => "连接 %{model}？",
        "cam.prompt_body" => "相机已开机，就在附近。",
        "cam.first_time" => "第一次连接？请先在相机上选择",
        "cam.first_time_device" => "第一次连接这台相机？请先在相机上选择",
        "cam.path_common" => "「配对」→「执行配对」",
        "cam.path_gr3" => "「蓝牙设定」→「配对」→「执行配对」",
        "cam.path_gr4" => "「无线通信设定」→「配对」→「执行配对」",
        "cam.step_pair" => "正在与相机配对",
        "cam.step_wake" => "正在连接相机",
        "cam.step_wake_sub" => "正在通过蓝牙唤醒相机。",
        "cam.step_pair_gr3" => "相机可能会显示配对码，iOS 弹窗出现时输入即可。",
        "cam.step_pair_gr4" => "相机可能会弹出配对确认，请在相机上同意。",
        "cam.step_wifi" => "正在加入相机 Wi‑Fi",
        "cam.step_wifi_sub" => "若 iOS 弹出提示，请允许 GRKit 连接相机。",
        "cam.step_wifi_slow" => "GR IV 启动 Wi‑Fi 需要更长时间，请稍候。",
        "cam.step_verify" => "正在与相机通信",
        "cam.step_verify_sub" => "正在检查连接。",
        "cam.no_grkit_title" => "相机上没有 GRKit",
        "cam.no_grkit_body" => "需要在存储卡上安装 GRKit 1.7.0 或更高版本。",
        "cam.install_guide" => "查看安装说明",
        "android.cam.step_wifi_sub" => "若系统弹出提示，请允许 GRKit 连接相机。",
        "android.cam.step_pair_gr3" => "相机可能会显示配对码，系统弹窗出现时输入即可。",
        _ => return None,
    })
}
