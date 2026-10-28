# The Linux side of the start: the card into RAM, then SCAN.
src=${1:-/media/storage1}
if mkdir /tmp/gc 2>/dev/null; then
  mount -t tmpfs -o size=16m tmpfs /tmp/gc
  mkdir -p /tmp/gc/SCRIPT/GRKIT /tmp/gc/GRKIT
  for f in PACKAGE.TXT SCAN.SH INSTALL.SH FINISH.SH CORE.BIN; do cp $src/SCRIPT/GRKIT/$f /tmp/gc/SCRIPT/GRKIT/; done
  chmod 755 /tmp/gc/SCRIPT/GRKIT/CORE*.BIN
  [ -d $src/SCRIPT/GRKIT/MODULES ] && cp -r $src/SCRIPT/GRKIT/MODULES /tmp/gc/SCRIPT/GRKIT/
  for f in STATE.TXT STATE.TMP SAFE SAFE.TXT PROFILE PROFILE.TXT IMPORTED.TXT REPORT.TXT RELOC.BIN PLAN.KEY; do [ -e $src/GRKIT/$f ] && cp $src/GRKIT/$f /tmp/gc/GRKIT/; done
  [ -d $src/GRKIT/MODULES ] && cp -r $src/GRKIT/MODULES /tmp/gc/GRKIT/
  # The state of GRKit 1.2, which INSTALL reads when GRKIT has no state file.
  for f in STATE.TXT STATE.TMP; do [ -e $src/SCRIPT/PLUGINS/$f ] && mkdir -p /tmp/gc/SCRIPT/PLUGINS && cp $src/SCRIPT/PLUGINS/$f /tmp/gc/SCRIPT/PLUGINS/; done
  # The plan of an earlier start, and the listing of the folders that keys it.
  [ -e $src/GRKIT/PLAN.BIN ] && cp $src/GRKIT/PLAN.BIN /tmp/gc/SCRIPT/GRKIT/PLAN.BIN
  dirs=
  for d in $src/GRKIT/EFFECTS $src/SCRIPT/GRKIT/EFFECTS $src/GRKIT/MODULES $src/SCRIPT/GRKIT/MODULES; do [ -d $d ] && dirs="$dirs $d"; done
  { echo $dirs; ls -le $dirs || ls -l $dirs; } >/tmp/gc/GRKIT/LIST.TXT 2>&1
  sh /tmp/gc/SCRIPT/GRKIT/SCAN.SH
  if [ $? = 49 ]; then
    # A new plan: the effect folders, and the effects of GRKit 1.2 when GRKIT has no library.
    mkdir -p /tmp/gc/SCRIPT/GRKIT/EFFECTS
    for d in EFFECTS; do [ -d $src/SCRIPT/GRKIT/$d ] && cp -r $src/SCRIPT/GRKIT/$d /tmp/gc/SCRIPT/GRKIT/; done
    for d in EFFECTS; do [ -d $src/GRKIT/$d ] && cp -r $src/GRKIT/$d /tmp/gc/GRKIT/; done
    if [ ! -e $src/GRKIT/STATE.TXT ] || [ ! -d $src/GRKIT/EFFECTS ]; then
      [ -d $src/SCRIPT/PLUGINS/EFFECTS ] && mkdir -p /tmp/gc/SCRIPT/PLUGINS && cp -r $src/SCRIPT/PLUGINS/EFFECTS /tmp/gc/SCRIPT/PLUGINS/
    fi
    sh /tmp/gc/SCRIPT/GRKIT/SCAN.SH
  fi
fi
# Release the mount of the loader before the camera mounts the card. If
# the camera tried in the meantime, its mount failed: keep this one for
# it, because an unmount now would stop Linux.
if [ $src != /media/storage1 ]; then
  if grep -q 'mnt /media/storage1' /var/log/messages && ! grep -q storage1 /proc/mounts; then
    mount --move $src /media/storage1
  else
    umount $src
  fi
  # A file in the mount point keeps it, so a retry of the loader mounts no more.
  grep -q " $src " /proc/mounts || touch $src/staged
fi
exit 0
