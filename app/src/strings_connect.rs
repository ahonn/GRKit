//! Connection error copy in the five app languages. Keys match `locales/app.yml`.

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
        "cam.more" => "More",
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
        "cam.more" => "更多",
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

pub fn zh_hant(key: &str) -> Option<&'static str> {
    Some(match key {
        "conn.app_old.title" => "需要更新 GRKit App",
        "conn.app_old.body" => "相機上的 GRKit 比這個 App 新。請先更新 App，再連線。",
        "conn.unsupported.body" => "相機有回應，但 GRKit 無法辨識。請重試。",
        "conn.open_settings" => "開啟設定",
        "conn.switched_wifi" => "已切換 Wi‑Fi",
        "conn.ble_failed.title" => "藍牙連線失敗",
        "conn.ble_failed.body" => "請關閉藍牙再重新開啟，然後重試。",
        "conn.ble_auth.title" => "配對未完成",
        "conn.ble_auth.body" => {
            "相機沒有完成配對。請在相機上選擇「配對」→「執行配對」，然後重試。如果仍然失敗，請先在「設定」→「藍牙」中忘記此相機。"
        }
        "conn.ble_pair.title" => "請在相機上執行藍牙配對",
        "conn.ble_pair.body" => "請在相機上選擇「配對」→「執行配對」，然後點一下「重試」。",
        "conn.ble_stale.title" => "相機已不記得此裝置",
        "conn.ble_stale.body" => {
            "請在「設定」→「藍牙」中點一下此相機旁的 ⓘ，選擇「忘記此裝置」。然後在相機上重新配對並重試。"
        }
        "conn.ble_timeout.title" => "相機沒有回應",
        "conn.ble_timeout.body" => "請開啟相機電源並放在附近，然後重試。",
        "conn.wifi_denied.title" => "已取消加入 Wi‑Fi",
        "conn.wifi_denied.body" => {
            "請在系統提示中點選「加入」，或在「設定」→「Wi‑Fi」中加入相機 Wi‑Fi。"
        }
        "conn.wifi_password.title" => "Wi‑Fi 密碼錯誤",
        "conn.wifi_password.body" => "相機的 Wi‑Fi 密碼可能已變更。請在相機上重新配對，然後重試。",
        "conn.wifi_ssid.title" => "Wi‑Fi 名稱無效",
        "conn.wifi_ssid.body" => "請檢查相機的 SSID。",
        "conn.wifi_config.title" => "Wi‑Fi 設定被拒絕",
        "conn.wifi_config.body" => "iOS 無法套用此 Wi‑Fi 設定，請重試。",
        "android.conn.wifi_config.body" => "Android 無法套用此 Wi‑Fi 設定，請重試。",
        "conn.wifi_failed.title" => "相機 Wi‑Fi 尚未就緒",
        "conn.wifi_failed.body" => "相機的 Wi‑Fi 還沒有啟動。請保持相機開機幾秒鐘，然後重試。",
        "conn.wifi_timeout.title" => "無法連線相機 Wi‑Fi",
        "conn.wifi_timeout.body" => "請確認相機已開機且 Wi‑Fi 已開啟。",
        "conn.network_not_ready.title" => "正在切換到相機 Wi‑Fi",
        "conn.network_not_ready.body" => {
            "裝置已加入相機 Wi‑Fi，但還沒有切換過去。請稍候片刻後重試。"
        }
        "conn.subnet.title" => "Wi‑Fi 網段與相機衝突",
        "conn.subnet.body" => {
            "手機連到了路由器，不是相機（兩者都用 192.168.0.x）。請開啟飛航模式，只開 Wi‑Fi，再加入相機 Wi‑Fi。"
        }
        "conn.verify.title" => "相機沒有回應",
        "conn.verify.body" => "已加入 Wi‑Fi 但相機沒有回應，請重新啟動相機。",
        "conn.local_network.title" => "本機網路存取被拒絕",
        "conn.local_network.body" => {
            "GRKit 需要本機網路權限才能與相機通訊。請前往「設定」→「GRKit」→「本機網路」開啟。"
        }
        "android.conn.local_network.body" => {
            "GRKit 需要「鄰近裝置」權限才能透過 Wi‑Fi 與相機通訊。請輕點「開啟設定」，然後在「權限」→「鄰近裝置」中允許。"
        }
        "conn.not_gr.title" => "不是 GR 相機",
        "conn.not_gr.body" => "看起來你連線到了其他 Wi‑Fi，請切換到相機的 Wi‑Fi。",
        "conn.unsupported.title" => "無法辨識的相機回應",
        "conn.unknown.title" => "連線失敗",
        "conn.unknown.body" => "發生問題，請重試。",
        "cam.title" => "相機",
        "cam.detect" => "已加入相機 Wi‑Fi？立即偵測",
        "cam.nearby" => "已開機 · 在附近",
        "cam.others" => "其他相機",
        "cam.lost_title" => "連線已中斷",
        "cam.current_none" => "未使用配方或底片風格",
        "cam.remove" => "移除相機",
        "cam.remove_body" => "確定從已儲存的相機中移除「%{name}」嗎？",
        "cam.remove_confirm" => "移除",
        "cam.cancel" => "取消",
        "cam.connected" => "已連線",
        "cam.saved" => "已儲存的相機",
        "cam.saved_note" => "一次只能連線一台相機。",
        "cam.settings_group" => "相機",
        "cam.help" => "說明與意見回饋",
        "cam.disconnect" => "中斷連線",
        "cam.more" => "更多",
        "cam.disconnect_title" => "中斷相機連線",
        "cam.disconnect_body" => "相機 Wi‑Fi 會關閉，手機連回原來的網路。",
        "cam.disconnect_confirm" => "中斷",
        "cam.power" => "關閉相機",
        "cam.power_title" => "關閉相機",
        "cam.power_body" => "相機會關機。",
        "cam.power_confirm" => "關機",
        "cam.prompt_title" => "連線 %{model}？",
        "cam.prompt_body" => "相機已開機，就在附近。",
        "cam.first_time" => "第一次連線？請先在相機上選擇",
        "cam.first_time_device" => "第一次連線這台相機？請先在相機上選擇",
        "cam.path_common" => "「配對」→「執行配對」",
        "cam.path_gr3" => "「Bluetooth 設定」→「配對」→「執行配對」",
        "cam.path_gr4" => "「無線通訊設定」→「配對」→「執行配對」",
        "cam.step_pair" => "正在與相機配對",
        "cam.step_wake" => "正在連線相機",
        "cam.step_wake_sub" => "正在透過藍牙喚醒相機。",
        "cam.step_pair_gr3" => "相機可能會顯示配對碼，iOS 跳出提示時輸入即可。",
        "cam.step_pair_gr4" => "相機可能會跳出配對確認，請在相機上同意。",
        "cam.step_wifi" => "正在加入相機 Wi‑Fi",
        "cam.step_wifi_sub" => "若 iOS 顯示提示，請允許 GRKit 連接相機。",
        "cam.step_wifi_slow" => "GR IV 啟動 Wi‑Fi 需要更長時間，請稍候。",
        "cam.step_verify" => "正在與相機通訊",
        "cam.step_verify_sub" => "正在檢查連線。",
        "cam.no_grkit_title" => "相機上沒有 GRKit",
        "cam.no_grkit_body" => "需要在記憶卡上安裝 GRKit 1.7.0 或更新版本。",
        "cam.install_guide" => "查看安裝說明",
        "android.cam.step_wifi_sub" => "若 Android 顯示提示，請允許 GRKit 連接相機。",
        "android.cam.step_pair_gr3" => "相機可能會顯示配對碼，Android 跳出提示時輸入即可。",
        _ => return None,
    })
}

pub fn ja(key: &str) -> Option<&'static str> {
    Some(match key {
        "conn.app_old.title" => "GRKit アプリのアップデートが必要",
        "conn.app_old.body" => {
            "カメラの GRKit はこのアプリより新しいバージョンです。アプリをアップデートしてから、もう一度接続してください。"
        }
        "conn.unsupported.body" => {
            "カメラは応答しましたが、GRKit がその内容を理解できませんでした。もう一度お試しください。"
        }
        "conn.open_settings" => "設定を開く",
        "conn.switched_wifi" => "Wi‑Fi を切り替えました",
        "conn.ble_failed.title" => "Bluetooth 接続に失敗しました",
        "conn.ble_failed.body" => {
            "Bluetooth をオフにしてから再度オンにし、もう一度お試しください。"
        }
        "conn.ble_auth.title" => "ペアリングが完了しませんでした",
        "conn.ble_auth.body" => {
            "カメラ側でペアリングが完了しませんでした。カメラで「ペアリング」→「ペアリング実行」を選択し、もう一度お試しください。それでも失敗する場合は、先に「設定」→「Bluetooth」でこのカメラの登録を解除してください。"
        }
        "conn.ble_pair.title" => "カメラで Bluetooth ペアリングを開始してください",
        "conn.ble_pair.body" => {
            "カメラで「ペアリング」→「ペアリング実行」を選択してから、「再試行」をタップしてください。"
        }
        "conn.ble_stale.title" => "カメラがこの端末を認識していません",
        "conn.ble_stale.body" => {
            "「設定」→「Bluetooth」でこのカメラの横の「ⓘ」をタップし、「このデバイスの登録を解除」を選択してください。その後、カメラでもう一度ペアリングしてから再試行してください。"
        }
        "conn.ble_timeout.title" => "カメラが応答しません",
        "conn.ble_timeout.body" => "カメラの電源を入れて近くに置き、もう一度お試しください。",
        "conn.wifi_denied.title" => "Wi‑Fi 接続をキャンセルしました",
        "conn.wifi_denied.body" => {
            "システムの確認で「接続」をタップするか、「設定」→「Wi‑Fi」でカメラの Wi‑Fi に接続してください。"
        }
        "conn.wifi_password.title" => "Wi‑Fi パスワードが違います",
        "conn.wifi_password.body" => {
            "カメラの Wi‑Fi パスワードが変更された可能性があります。カメラでもう一度ペアリングしてから、再試行してください。"
        }
        "conn.wifi_ssid.title" => "Wi‑Fi 名が無効です",
        "conn.wifi_ssid.body" => "カメラの SSID を確認してください。",
        "conn.wifi_config.title" => "Wi‑Fi 設定が拒否されました",
        "conn.wifi_config.body" => {
            "iOS がこの Wi‑Fi 設定を適用できませんでした。もう一度お試しください。"
        }
        "android.conn.wifi_config.body" => {
            "Android がこの Wi‑Fi 設定を適用できませんでした。もう一度お試しください。"
        }
        "conn.wifi_failed.title" => "カメラの Wi‑Fi の準備ができていません",
        "conn.wifi_failed.body" => {
            "カメラの Wi‑Fi がまだ起動していません。カメラの電源を入れたまま数秒待ってから、もう一度お試しください。"
        }
        "conn.wifi_timeout.title" => "カメラの Wi‑Fi に接続できません",
        "conn.wifi_timeout.body" => {
            "カメラの電源が入っていて、Wi‑Fi がオンになっているか確認してください。"
        }
        "conn.network_not_ready.title" => "カメラの Wi‑Fi に切り替え中です",
        "conn.network_not_ready.body" => {
            "お使いの端末はカメラの Wi‑Fi に接続しましたが、まだ切り替わっていません。少し待ってから、もう一度お試しください。"
        }
        "conn.subnet.title" => "Wi‑Fi のアドレスが重複しています",
        "conn.subnet.body" => {
            "スマートフォンがカメラではなくルーターに接続しています（どちらも 192.168.0.x を使用）。機内モードをオンにしてから Wi‑Fi だけをオンに戻し、カメラの Wi‑Fi に接続してください。"
        }
        "conn.verify.title" => "カメラが応答しません",
        "conn.verify.body" => {
            "Wi‑Fi に接続しましたが応答がありません。カメラを再起動してください。"
        }
        "conn.local_network.title" => "ローカルネットワークへのアクセスが拒否されました",
        "conn.local_network.body" => {
            "カメラと通信するには「ローカルネットワーク」の許可が必要です。「設定」→「GRKit」→「ローカルネットワーク」で有効にしてください。"
        }
        "android.conn.local_network.body" => {
            "Wi‑Fi でカメラと通信するには「付近のデバイス」の権限が必要です。「設定を開く」をタップし、「権限」→「付近のデバイス」で許可してください。"
        }
        "conn.not_gr.title" => "GR カメラではありません",
        "conn.not_gr.body" => {
            "別の Wi‑Fi ネットワークに接続されているようです。カメラの Wi‑Fi に切り替えてください。"
        }
        "conn.unsupported.title" => "未対応のカメラ応答",
        "conn.unknown.title" => "接続できませんでした",
        "conn.unknown.body" => "問題が発生しました。もう一度お試しください。",
        "cam.title" => "カメラ",
        "cam.detect" => "カメラの Wi‑Fi に接続済み？今すぐ検出",
        "cam.nearby" => "電源オン · 近くにあります",
        "cam.others" => "その他のカメラ",
        "cam.lost_title" => "接続が切れました",
        "cam.current_none" => "レシピもフィルムルックも未使用",
        "cam.remove" => "カメラを削除",
        "cam.remove_body" => "保存済みのカメラから「%{name}」を削除しますか？",
        "cam.remove_confirm" => "削除",
        "cam.cancel" => "キャンセル",
        "cam.connected" => "接続済み",
        "cam.saved" => "保存済みのカメラ",
        "cam.saved_note" => "同時に接続できるカメラは 1 台だけです。",
        "cam.settings_group" => "カメラ",
        "cam.help" => "ヘルプとフィードバック",
        "cam.disconnect" => "接続を解除",
        "cam.more" => "その他",
        "cam.disconnect_title" => "カメラとの接続を解除",
        "cam.disconnect_body" => {
            "カメラの Wi‑Fi がオフになり、スマートフォンは元のネットワークに戻ります。"
        }
        "cam.disconnect_confirm" => "解除",
        "cam.power" => "カメラの電源を切る",
        "cam.power_title" => "カメラの電源を切る",
        "cam.power_body" => "カメラの電源が切れます。",
        "cam.power_confirm" => "電源を切る",
        "cam.prompt_title" => "%{model} に接続しますか？",
        "cam.prompt_body" => "カメラの電源が入っていて、近くにあります。",
        "cam.first_time" => "初めて接続しますか？先にカメラで選択：",
        "cam.first_time_device" => "このカメラに初めて接続しますか？先にカメラで選択：",
        "cam.path_common" => "「ペアリング」→「ペアリング実行」",
        "cam.path_gr3" => "「Bluetooth設定」→「ペアリング」→「ペアリング実行」",
        "cam.path_gr4" => "「無線通信設定」→「ペアリング」→「ペアリング実行」",
        "cam.step_pair" => "カメラとペアリング中",
        "cam.step_wake" => "カメラに接続中",
        "cam.step_wake_sub" => "Bluetooth でカメラを起動しています。",
        "cam.step_pair_gr3" => {
            "カメラにペアリングコードが表示されることがあります。iOS の入力画面が出たら入力してください。"
        }
        "cam.step_pair_gr4" => {
            "カメラに確認画面が表示されることがあります。カメラ側で承認してください。"
        }
        "cam.step_wifi" => "カメラの Wi‑Fi に接続中",
        "cam.step_wifi_sub" => "iOS で確認が表示されたら、GRKit とカメラの接続を許可してください。",
        "cam.step_wifi_slow" => "GR IV は Wi‑Fi の起動に時間がかかります。お待ちください。",
        "cam.step_verify" => "カメラと通信中",
        "cam.step_verify_sub" => "接続を確認しています。",
        "cam.no_grkit_title" => "カメラに GRKit がありません",
        "cam.no_grkit_body" => "メモリーカードに GRKit 1.7.0 以降をインストールしてください。",
        "cam.install_guide" => "インストールガイド",
        "android.cam.step_wifi_sub" => {
            "Android で確認が表示されたら、GRKit とカメラの接続を許可してください。"
        }
        "android.cam.step_pair_gr3" => {
            "カメラにペアリングコードが表示されることがあります。Android の入力画面が出たら入力してください。"
        }
        _ => return None,
    })
}

pub fn ko(key: &str) -> Option<&'static str> {
    Some(match key {
        "conn.app_old.title" => "GRKit 앱 업데이트 필요",
        "conn.app_old.body" => {
            "카메라의 GRKit이 이 앱보다 최신입니다. 앱을 업데이트한 후 다시 연결하세요."
        }
        "conn.unsupported.body" => {
            "카메라가 응답했지만 GRKit이 응답 내용을 이해하지 못했습니다. 다시 시도하세요."
        }
        "conn.open_settings" => "설정 열기",
        "conn.switched_wifi" => "Wi‑Fi 전환 완료",
        "conn.ble_failed.title" => "Bluetooth 연결 실패",
        "conn.ble_failed.body" => "Bluetooth를 껐다 켠 후 다시 시도하세요.",
        "conn.ble_auth.title" => "페어링이 완료되지 않음",
        "conn.ble_auth.body" => {
            "카메라에서 페어링이 끝나지 않았습니다. 카메라에서 「페어링」 → 「페어링 실행」을 선택한 후 다시 시도하세요. 그래도 실패하면 먼저 설정 → Bluetooth에서 이 카메라를 지우세요."
        }
        "conn.ble_pair.title" => "카메라에서 Bluetooth 페어링 시작",
        "conn.ble_pair.body" => {
            "카메라에서 「페어링」 → 「페어링 실행」을 선택한 후 「다시 시도」를 탭하세요."
        }
        "conn.ble_stale.title" => "카메라가 이 기기를 기억하지 못함",
        "conn.ble_stale.body" => {
            "설정 → Bluetooth에서 이 카메라 옆의 ⓘ를 탭하고 「이 기기 지우기」를 선택하세요. 그런 다음 카메라에서 다시 페어링한 후 다시 시도하세요."
        }
        "conn.ble_timeout.title" => "카메라가 응답하지 않음",
        "conn.ble_timeout.body" => "카메라를 켜고 가까이 둔 후 다시 시도하세요.",
        "conn.wifi_denied.title" => "Wi‑Fi 연결 취소됨",
        "conn.wifi_denied.body" => {
            "시스템 알림에서 「연결」을 탭하거나 설정 → Wi‑Fi에서 카메라 Wi‑Fi에 연결하세요."
        }
        "conn.wifi_password.title" => "잘못된 Wi‑Fi 비밀번호",
        "conn.wifi_password.body" => {
            "카메라의 Wi‑Fi 비밀번호가 변경되었을 수 있습니다. 카메라에서 다시 페어링한 후 다시 시도하세요."
        }
        "conn.wifi_ssid.title" => "잘못된 Wi‑Fi 이름",
        "conn.wifi_ssid.body" => "카메라의 SSID를 확인하세요.",
        "conn.wifi_config.title" => "Wi‑Fi 설정이 거부됨",
        "conn.wifi_config.body" => "iOS가 이 Wi‑Fi 설정을 적용하지 못했습니다. 다시 시도하세요.",
        "android.conn.wifi_config.body" => {
            "Android가 이 Wi‑Fi 설정을 적용하지 못했습니다. 다시 시도하세요."
        }
        "conn.wifi_failed.title" => "카메라 Wi‑Fi가 아직 준비되지 않음",
        "conn.wifi_failed.body" => {
            "카메라 Wi‑Fi가 아직 시작되지 않았습니다. 카메라를 켠 채 몇 초 기다린 후 다시 시도하세요."
        }
        "conn.wifi_timeout.title" => "카메라 Wi‑Fi에 연결할 수 없음",
        "conn.wifi_timeout.body" => "카메라가 켜져 있고 Wi‑Fi가 활성화되어 있는지 확인하세요.",
        "conn.network_not_ready.title" => "카메라 Wi‑Fi로 전환하는 중",
        "conn.network_not_ready.body" => {
            "기기가 카메라 Wi‑Fi에 연결되었지만 아직 전환되지 않았습니다. 잠시 기다린 후 다시 시도하세요."
        }
        "conn.subnet.title" => "Wi‑Fi 주소 대역이 겹칩니다",
        "conn.subnet.body" => {
            "휴대폰이 카메라 대신 공유기에 연결되었습니다(둘 다 192.168.0.x 사용). 에어플레인 모드를 켠 뒤 Wi‑Fi만 다시 켜고 카메라 Wi‑Fi에 연결하세요."
        }
        "conn.verify.title" => "카메라가 응답하지 않음",
        "conn.verify.body" => "Wi‑Fi에 연결되었지만 응답이 없습니다. 카메라를 재시작하세요.",
        "conn.local_network.title" => "로컬 네트워크 접근 거부됨",
        "conn.local_network.body" => {
            "GRKit이 카메라와 통신하려면 로컬 네트워크 접근 권한이 필요합니다. 설정 → GRKit → 로컬 네트워크에서 허용하세요."
        }
        "android.conn.local_network.body" => {
            "GRKit이 Wi‑Fi로 카메라와 통신하려면 근처 기기 권한이 필요합니다. 「설정 열기」를 탭한 다음 권한 → 근처 기기에서 허용하세요."
        }
        "conn.not_gr.title" => "GR 카메라가 아님",
        "conn.not_gr.body" => {
            "다른 Wi‑Fi 네트워크에 연결된 것 같습니다. 카메라의 Wi‑Fi로 전환하세요."
        }
        "conn.unsupported.title" => "지원되지 않는 카메라 응답",
        "conn.unknown.title" => "연결할 수 없음",
        "conn.unknown.body" => "문제가 발생했습니다. 다시 시도해 주세요.",
        "cam.title" => "카메라",
        "cam.detect" => "카메라 Wi‑Fi에 연결했나요? 지금 감지",
        "cam.nearby" => "켜짐 · 근처에 있음",
        "cam.others" => "다른 카메라",
        "cam.lost_title" => "연결이 끊김",
        "cam.current_none" => "사용 중인 레시피나 필름 룩 없음",
        "cam.remove" => "카메라 삭제",
        "cam.remove_body" => "저장된 카메라에서 「%{name}」을(를) 삭제할까요?",
        "cam.remove_confirm" => "삭제",
        "cam.cancel" => "취소",
        "cam.connected" => "연결됨",
        "cam.saved" => "저장된 카메라",
        "cam.saved_note" => "한 번에 카메라 한 대만 연결할 수 있습니다.",
        "cam.settings_group" => "카메라",
        "cam.help" => "도움말 및 피드백",
        "cam.disconnect" => "연결 해제",
        "cam.more" => "더 보기",
        "cam.disconnect_title" => "카메라 연결 해제",
        "cam.disconnect_body" => "카메라 Wi‑Fi가 꺼지고 휴대폰은 이전 네트워크로 돌아갑니다.",
        "cam.disconnect_confirm" => "해제",
        "cam.power" => "카메라 끄기",
        "cam.power_title" => "카메라 끄기",
        "cam.power_body" => "카메라가 꺼집니다.",
        "cam.power_confirm" => "끄기",
        "cam.prompt_title" => "%{model}에 연결할까요?",
        "cam.prompt_body" => "카메라가 켜져 있고 근처에 있습니다.",
        "cam.first_time" => "처음 연결하나요? 먼저 카메라에서 선택:",
        "cam.first_time_device" => "이 카메라에 처음 연결하나요? 먼저 카메라에서 선택:",
        "cam.path_common" => "「페어링」 → 「페어링 실행」",
        "cam.path_gr3" => "「Bluetooth 설정」 → 「페어링」 → 「페어링 실행」",
        "cam.path_gr4" => "「무선 통신 설정」 → 「페어링」 → 「페어링 실행」",
        "cam.step_pair" => "카메라와 페어링 중",
        "cam.step_wake" => "카메라에 연결하는 중",
        "cam.step_wake_sub" => "Bluetooth로 카메라를 깨우는 중입니다.",
        "cam.step_pair_gr3" => {
            "카메라에 페어링 코드가 표시될 수 있습니다. iOS 입력 창이 나타나면 입력해 주세요."
        }
        "cam.step_pair_gr4" => "카메라에 확인 창이 표시될 수 있습니다. 카메라에서 승인해 주세요.",
        "cam.step_wifi" => "카메라 Wi‑Fi에 연결 중",
        "cam.step_wifi_sub" => "iOS에서 요청하면 GRKit의 카메라 연결을 허용해 주세요.",
        "cam.step_wifi_slow" => {
            "GR IV는 Wi‑Fi를 시작하는 데 시간이 더 걸립니다. 잠시 기다려 주세요."
        }
        "cam.step_verify" => "카메라와 통신하는 중",
        "cam.step_verify_sub" => "연결을 확인하는 중입니다.",
        "cam.no_grkit_title" => "카메라에 GRKit이 없음",
        "cam.no_grkit_body" => "메모리 카드에 GRKit 1.7.0 이상을 설치하세요.",
        "cam.install_guide" => "설치 안내",
        "android.cam.step_wifi_sub" => "Android에서 요청하면 GRKit의 카메라 연결을 허용해 주세요.",
        "android.cam.step_pair_gr3" => {
            "카메라에 페어링 코드가 표시될 수 있습니다. Android 입력 창이 나타나면 입력해 주세요."
        }
        _ => return None,
    })
}
