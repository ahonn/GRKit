read t _ </proc/uptime; echo "$t install-start" >>/media/storage1/GRKIT/BOOT.TXT
/media/storage1/SCRIPT/GRKIT/CORE.BIN --install
status=$?
read t _ </proc/uptime; echo "$t install $status" >>/media/storage1/GRKIT/BOOT.TXT
echo GR_MODULE_INSTALL_EXIT_$status
exit 0
