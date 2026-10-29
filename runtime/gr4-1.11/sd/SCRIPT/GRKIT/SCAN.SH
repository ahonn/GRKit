read t _ </proc/uptime; echo "$t scan-start" >>/tmp/gc/BOOT.TXT
/tmp/gc/SCRIPT/GRKIT/CORE.BIN --scan /tmp/gc
status=$?
read t _ </proc/uptime; echo "$t scan $status" >>/tmp/gc/BOOT.TXT
echo GR_MODULE_SCAN_EXIT_$status
exit $status
