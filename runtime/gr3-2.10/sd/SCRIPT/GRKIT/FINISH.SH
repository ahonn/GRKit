# After INSTALL: the results to the card, once the camera mounts it.
(
  n=0
  while ! grep -q storage1 /proc/mounts; do
    usleep 200000
    n=$((n + 1))
    [ $n -gt 300 ] && exit 0
  done
  for f in INSTALL.OK INSTALL.ERR; do [ -e /tmp/gc/SCRIPT/GRKIT/$f ] && cp /tmp/gc/SCRIPT/GRKIT/$f /media/storage1/SCRIPT/GRKIT/; done
  mkdir -p /media/storage1/GRKIT
  # The plan and its key, for the next start, then the other results: each only on a change.
  [ -e /tmp/gc/SCRIPT/GRKIT/PLAN.BIN ] && { cmp -s /tmp/gc/SCRIPT/GRKIT/PLAN.BIN /media/storage1/GRKIT/PLAN.BIN || cp /tmp/gc/SCRIPT/GRKIT/PLAN.BIN /media/storage1/GRKIT/; }
  for f in PLAN.KEY REPORT.TXT RELOC.BIN; do
    [ -e /tmp/gc/GRKIT/$f ] || continue
    cmp -s /tmp/gc/GRKIT/$f /media/storage1/GRKIT/$f || cp /tmp/gc/GRKIT/$f /media/storage1/GRKIT/
  done
  /media/storage1/SCRIPT/GRKIT/CORE.BIN --import /media/storage1
  umount /tmp/gc
  rm -rf /tmp/gc
) </dev/null >/dev/null 2>&1 &
exit 0
