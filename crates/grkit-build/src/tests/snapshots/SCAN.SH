case "$1" in
serve)
  if [ ! -e /tmp/grkit-serve ] && [ -e /media/storage1/SCRIPT/GRKIT/SERVE.BIN ]; then
    : >/tmp/grkit-serve
    (n=0
    while :; do
      (cd / && exec /media/storage1/SCRIPT/GRKIT/SERVE.BIN) &
      echo $! >/tmp/grkit-serve
      wait $! && break
      n=$((n + 1))
      [ $n -gt 1 ] && break
      sleep 1
      [ -e /tmp/grkit-serve ] || break
    done
    rm -f /tmp/grkit-serve) </dev/null >/dev/null 2>&1 &
  fi
  exit 0 ;;
stop)
  if [ -e /tmp/grkit-serve ] && read pid _ </tmp/grkit-serve && [ -e /proc/$pid ]; then
    grep -q -- --from-ram /proc/$pid/cmdline && kill -TERM "$pid"
  else
    rm -f /tmp/grkit-serve
    [ -e /media/storage1/SCRIPT/GRKIT/SERVE.BIN ] && /media/storage1/SCRIPT/GRKIT/SERVE.BIN --mark-stopped
  fi
  exit 0 ;;
esac
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
