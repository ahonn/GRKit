# After INSTALL: the results to the card, once the camera mounts it.
(
  n=0
  while ! grep -q storage1 /proc/mounts; do
    usleep 200000
    n=$((n + 1))
    [ $n -gt 300 ] && exit 0
  done
  log() { read t _ </proc/uptime; echo "$t $*" >>/tmp/gc/BOOT.TXT; }
  # Seconds with two decimals in centiseconds (LOAD.SH: now).
  cs() { _s=${1%.*}; _f=${1#*.}; echo $((_s * 100 + 1$_f - 100)); }
  log finish
  for f in INSTALL.OK INSTALL.ERR; do [ -e /tmp/gc/SCRIPT/GRKIT/$f ] && cp /tmp/gc/SCRIPT/GRKIT/$f /media/storage1/SCRIPT/GRKIT/; done
  mkdir -p /media/storage1/GRKIT
  # The addresses of a search first, then the plan and its key, for the next start, then the
  # other results: each only on a change. A kept plan without the addresses would search
  # again, and a key without its plan would not keep it.
  [ -e /tmp/gc/GRKIT/RELOC.BIN ] && { cmp -s /tmp/gc/GRKIT/RELOC.BIN /media/storage1/GRKIT/RELOC.BIN || cp /tmp/gc/GRKIT/RELOC.BIN /media/storage1/GRKIT/; }
  [ -e /tmp/gc/SCRIPT/GRKIT/PLAN.BIN ] && { cmp -s /tmp/gc/SCRIPT/GRKIT/PLAN.BIN /media/storage1/GRKIT/PLAN.BIN || cp /tmp/gc/SCRIPT/GRKIT/PLAN.BIN /media/storage1/GRKIT/; }
  for f in PLAN.KEY REPORT.TXT; do
    [ -e /tmp/gc/GRKIT/$f ] || continue
    cmp -s /tmp/gc/GRKIT/$f /media/storage1/GRKIT/$f || cp /tmp/gc/GRKIT/$f /media/storage1/GRKIT/
  done
  # INSTALL refused the plan: the next start makes it again.
  e=
  [ -e /tmp/gc/SCRIPT/GRKIT/INSTALL.ERR ] && read e _ </tmp/gc/SCRIPT/GRKIT/INSTALL.ERR
  [ "$e" = 37 ] && rm -f /media/storage1/GRKIT/PLAN.KEY
  /media/storage1/SCRIPT/GRKIT/CORE.BIN --import /media/storage1
  # The fast path: off after a start near the camera's mount or without an
  # install, on after a slow start with time to spare.
  fast=; loaded=; first=; released=; moved=; tried=
  while read t step arg; do
    case $step in
      load) loaded=$t; [ "$arg" = /media/storage1 ] || fast=1 ;;
      scan) [ -n "$first" ] || first=$t ;;
      released) released=$t; [ "$arg" = move ] && moved=1 ;;
      camera-try) tried=$t ;;
    esac
  done </tmp/gc/BOOT.TXT
  if [ -n "$fast" ]; then
    near=
    [ -e /tmp/gc/SCRIPT/GRKIT/INSTALL.OK ] || near=1
    [ -n "$moved" ] && near=1
    [ -n "$tried" ] && [ -n "$released" ] && [ $(($(cs $tried) - $(cs $released))) -lt 30 ] && near=1
    [ -n "$near" ] && { : >/media/storage1/SCRIPT/GRKIT/FAST.OFF; log fast-off; }
  elif [ -e /tmp/t0 ] && [ -n "$first" ] && [ -e /tmp/gc/SCRIPT/GRKIT/INSTALL.OK ] && [ ! -e /media/storage1/SCRIPT/GRKIT/FAST.OFF ]; then
    # LOAD.SH started at most one request interval after the camera's mount.
    read a _ </tmp/t0
    w=$(($(cs $loaded) - $(cs $a) - 30))
    h=$(($(cs $first) - $(cs $loaded) + 10))
    [ $w -ge 110 ] && { echo $w >/media/storage1/SCRIPT/GRKIT/FAST.ON; log fast-on $w $h; }
  fi
  rm -f /media/storage1/SCRIPT/GRKIT/FAST.TRY
  # The rest of the boot log, with the failed mounts of the camera, and only its last lines.
  log done $(grep -c 'fail to mount' /var/log/messages)
  b=/media/storage1/GRKIT/BOOT.TXT
  n=$(cat /tmp/gc/BOOT.N 2>/dev/null)
  tail -n +$((${n:-0} + 1)) /tmp/gc/BOOT.TXT >>$b
  [ $(grep -c '' $b) -gt 400 ] && tail -n 300 $b >/tmp/gc/b && cp /tmp/gc/b $b
  umount /tmp/gc
  rm -rf /tmp/gc
) </dev/null >/dev/null 2>&1 &
exit 0
