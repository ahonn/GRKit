mkdir -p /media/storage1/GRKIT
if [ ! -e /tmp/grkit-boot ]; then
  : >/tmp/grkit-boot
  n=$(grep -c '' /media/storage1/GRKIT/BOOT.TXT 2>/dev/null)
  [ "${n:-0}" -gt 400 ] && tail -n 300 /media/storage1/GRKIT/BOOT.TXT >/tmp/grkit-boot && cat /tmp/grkit-boot >/media/storage1/GRKIT/BOOT.TXT
  read t _ </proc/uptime; echo "$t load" >>/media/storage1/GRKIT/BOOT.TXT
fi
read t _ </proc/uptime; echo "$t scan-start" >>/media/storage1/GRKIT/BOOT.TXT
/media/storage1/SCRIPT/GRKIT/CORE.BIN --scan
status=$?
read t _ </proc/uptime; echo "$t scan $status" >>/media/storage1/GRKIT/BOOT.TXT
echo GR_MODULE_SCAN_EXIT_$status
exit 0
