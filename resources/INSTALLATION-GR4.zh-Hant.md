# GRKit

僅適用於 RICOH GR IV 與 GR IV HDF 韌體 1.11。

GRKit 是實驗性軟體，不提供任何擔保。如果 GRKit 出錯，相機可能在啟動時停止回應。請在安裝前閱讀[復原指南](https://github.com/ahonn/grkit/blob/main/resources/RECOVERY.zh-Hant.md)。安裝 GRKit 即表示你接受[使用條款](https://github.com/ahonn/grkit/blob/main/resources/TERMS.zh-Hant.md)。

## 安裝

在電腦上的 Chrome 或 Edge 中開啟 [grkit.app](https://grkit.app/zh-hant/)，可以透過讀卡機直接把 GRKit 寫入記憶卡，代替第 2 步。

1. 關閉相機。
2. 把 `SD_ROOT` 中的內容（不是這個資料夾本身）複製到記憶卡根目錄。複製後，根目錄中應有 `SCRIPT` 資料夾。
3. 插入記憶卡。按住 MENU 按鈕開機，等待相機自動關機。
4. 正常開機。

第 3 步會開啟相機的 SD 腳本功能：相機顯示 Mode Set Command 和 Complete，然後自動關機。如果 SD 腳本功能已經開啟，跳過這一步。

![Mode Set Command 畫面顯示 Script - Enable 和 Complete。](images/modeset.png)

安裝後，可以從記憶卡根目錄刪除 `00078560.636` 與 `DEVELOP.MOD`。

## 使用

開機幾秒後，拍攝畫面頂部顯示 **GRKit**，表示載入完成。安裝或更新後的第一次開機會久一些。載入完成前拍的照片沒有 GRKit 效果。在拍攝模式下按 ADJ 桿，選擇第 6 項 GRK。

- **底片風格：** 反白一個效果即可在實時顯示中預覽，按 OK 保留。按 Fn 調整這個效果。不會改變相機的影像控制設定。這些效果是為 GR III 製作的，在 GR IV 上顏色可能不同。
- **構圖比例：** XPan、2.35:1、16:9 等比例。預設裁切 JPEG 照片，DNG 保持 3:2。按 Fn 可以改為僅輔助構圖，或設定遮罩。
- **輔助線：** 只顯示在螢幕上，不會出現在照片中。
- **加印日期：** 在照片右下角加印日期。按 Fn 選擇日期樣式和字體。
- **光暈：** 柔光和底片光暈，只套用於 JPEG 照片，拍攝畫面不預覽。
- **粒紋感：** 為照片加上粒紋。按 Fn 選擇粒紋大小。

全部設定也在 MENU 自訂頁（C）最後的 **GRKit** 小節中。每個模組一列，可以啟用或關閉模組，在**可選項**中勾選 ADJ 裡顯示的效果、比例或樣式，在 **ADJ 選單順序**中調整順序。關機後設定會保留。

## 更新

在電腦上的 Chrome 或 Edge 中開啟 [grkit.app](https://grkit.app/zh-hant/)，可以透過讀卡機直接更新記憶卡，代替第 2、3 步。

1. 關閉相機。
2. 從記憶卡刪除 `SCRIPT` 資料夾。
3. 把新安裝包 `SD_ROOT` 中的 `SCRIPT` 資料夾複製到記憶卡根目錄。
4. 正常開機，不需要按住 MENU。

設定和效果保存在 `GRKIT` 資料夾中，更新後仍在。從 GRKit 1.2 更新時，請用 grkit.app 更新，以保留設定和效果。

## 快速啟動（選用）

在官方韌體上，GRKit 要等相機讀完記憶卡，開機後十秒左右才載入。裝上快速啟動韌體後，開機後一到兩秒就能載入。同一個安裝包兩種韌體都能用。不裝快速啟動韌體，GRKit 的功能也一樣。

快速啟動韌體在 grkit.app 的[快速啟動頁面](https://grkit.app/zh-hant/fastboot/)製作：選擇從 Ricoh 官網下載的官方韌體，頁面會產生快速啟動韌體，然後像更新官方韌體一樣安裝。安裝需要更新相機韌體，更新中斷時相機可能無法開機。要復原，安裝同一版本的官方韌體即可。不支援 GR IV Monochrome。

## 疑難排解

### 按住 MENU 開機時，相機開始更新韌體

記憶卡根目錄中有韌體檔案。關閉相機，刪除根目錄中所有 `.bin` 檔案，然後重新執行安裝第 3 步。

### 相機沒有顯示 Mode Set Command

- 先按住 MENU 按鈕，再開機。
- 把 `SD_ROOT` 中的內容複製到記憶卡根目錄，不要複製到 `DCIM` 等資料夾中。

### ADJ 中沒有 GRK

1. 開機後等待幾秒，直到拍攝畫面頂部顯示 **GRKit**。
2. 確認記憶卡根目錄中有 `SCRIPT` 資料夾。
3. 如果記憶卡中有 `SCRIPT/GRKIT/INSTALL.ERR`，在相機選單中確認相機顯示的是支援的韌體版本。版本受支援時，請提交[裝置報告](https://github.com/ahonn/grkit/issues/new?template=device-report.yml)，並附上 `INSTALL.ERR`、`GRKIT/REPORT.TXT` 和 `GRKIT/BOOT.TXT`（如果有）。
4. GRKit 只能在 Ricoh 官方韌體和 grkit.app 的快速啟動韌體上運作。

### 格式化記憶卡後 GRKit 不運作

格式化會刪除 GRKit 和它的設定。格式化之前，把 `GRKIT` 資料夾複製到電腦。格式化之後，重新複製 `SD_ROOT` 中的內容，再把 `GRKIT` 資料夾複製回記憶卡根目錄。

## 移除 GRKit

在電腦上的 Chrome 或 Edge 中開啟 [grkit.app](https://grkit.app/zh-hant/)，可以透過讀卡機直接移除 GRKit，並可同時關閉 SD 腳本功能。

關閉相機，從記憶卡刪除 `SCRIPT` 和 `GRKIT` 資料夾。

SD 腳本功能會保持開啟。要關閉它：

1. 在 https://grkit.app 下載你的相機對應的關閉 SD 腳本的檔案。
2. 把 `SD_ROOT` 中的內容複製到記憶卡根目錄。
3. 插入記憶卡。按住 MENU 按鈕開機，等待相機自動關機。
4. 從記憶卡根目錄刪除 `00078560.636` 與 `DEVELOP.MOD`。

相機在啟動時停止回應時，請參閱[復原指南](https://github.com/ahonn/grkit/blob/main/resources/RECOVERY.zh-Hant.md)。

GRKit 與 Ricoh 無關。原始碼和授權：https://github.com/ahonn/grkit
