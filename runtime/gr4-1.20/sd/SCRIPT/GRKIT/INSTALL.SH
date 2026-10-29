read t _ </proc/uptime; echo "$t install-start" >>/tmp/gc/BOOT.TXT
/tmp/gc/SCRIPT/GRKIT/CORE.BIN --install /tmp/gc
status=$?
read t _ </proc/uptime; echo "$t install $status" >>/tmp/gc/BOOT.TXT
echo GR_MODULE_INSTALL_EXIT_$status
exit 0
