# The Linux side of the start: the card into RAM, then SCAN.
src=${1:-/media/storage1}
# The functions run in the shell of the script, so their variables start
# with an underscore: a function must not change a variable of its caller,
# such as the loop variable of put.
# A line of the boot log: the uptime, then the step.
log() { read _t _ </proc/uptime; echo "$_t $*" >>/tmp/gc/BOOT.TXT; }
# The uptime in centiseconds, in $now, without a subshell. An arithmetic
# error stops the script, and a number that starts with 0 is octal, so the
# hundredths get a leading 1.
now() { read _t _ </proc/uptime; _s=${_t%.*}; _f=${_t#*.}; now=$((_s * 100 + 1$_f - 100)); }
# Release the mount of the loader before the camera mounts the card. If
# the camera tried in the meantime, its mount failed: keep this one for
# it, because an unmount now would stop Linux. Then the camera's mount
# is the card.
release() {
  [ $src = /media/storage1 ] && return
  log release
  # The log so far on the card, because a failed release stops Linux, and
  # FAST.TRY, which the loader removes once this request returns.
  mkdir -p $src/GRKIT || log mkdir $?
  _n=$(grep -c '' /tmp/gc/BOOT.TXT)
  cat /tmp/gc/BOOT.TXT >>$src/GRKIT/BOOT.TXT || log append $?
  echo $_n >/tmp/gc/BOOT.N
  : >$src/SCRIPT/GRKIT/FAST.TRY
  if grep -q 'mnt /media/storage1' /var/log/messages && ! grep -q storage1 /proc/mounts; then
    mount --move $src /media/storage1
    log released move
  else
    umount $src
    log released umount
  fi
  src=/media/storage1
}
# The camera's mount of the card.
mounted() {
  _n=0
  while ! grep -q storage1 /proc/mounts; do
    usleep 200000
    _n=$((_n + 1))
    [ $_n -gt 300 ] && return 1
  done
  return 0
}
# On the loader's mount, a step after the deadline is late.
late() {
  [ $src = /media/storage1 ] && return 1
  now
  [ $now -lt $deadline ] && return 1
  log late
  return 0
}
# A file, or each file of a flat folder, into the folder $2, unless late.
# After a late step, the copy resumes and skips the files that it has.
put() {
  if [ -d "$1" ]; then
    mkdir -p "$2/${1##*/}"
    for _p in "$1"/*; do
      [ -e "$_p" ] || continue
      [ -n "$resume" ] && [ -e "$2/${1##*/}/${_p##*/}" ] && continue
      late && return 1
      cp "$_p" "$2/${1##*/}/"
    done
  else
    [ -n "$resume" ] && [ -e "$2/${1##*/}" ] && return 0
    late && return 1
    cp "$1" "$2/"
  fi
  return 0
}
# The files that SCAN reads, from the card into the RAM copy.
fetch() {
  mkdir -p /tmp/gc/SCRIPT/GRKIT /tmp/gc/GRKIT
  for _f in PACKAGE.TXT SCAN.SH INSTALL.SH FINISH.SH CORE.BIN; do put $src/SCRIPT/GRKIT/$_f /tmp/gc/SCRIPT/GRKIT || return 1; done
  chmod 755 /tmp/gc/SCRIPT/GRKIT/CORE*.BIN
  [ -d $src/SCRIPT/GRKIT/MODULES ] && { put $src/SCRIPT/GRKIT/MODULES /tmp/gc/SCRIPT/GRKIT || return 1; }
  for _f in STATE.TXT STATE.TMP SAFE SAFE.TXT PROFILE PROFILE.TXT IMPORTED.TXT REPORT.TXT RELOC.BIN PLAN.KEY; do [ -e $src/GRKIT/$_f ] && { put $src/GRKIT/$_f /tmp/gc/GRKIT || return 1; }; done
  [ -d $src/GRKIT/MODULES ] && { put $src/GRKIT/MODULES /tmp/gc/GRKIT || return 1; }
  # The state of GRKit 1.2, which INSTALL reads when GRKIT has no state file.
  for _f in STATE.TXT STATE.TMP; do [ -e $src/SCRIPT/PLUGINS/$_f ] && { mkdir -p /tmp/gc/SCRIPT/PLUGINS; put $src/SCRIPT/PLUGINS/$_f /tmp/gc/SCRIPT/PLUGINS || return 1; }; done
  # The plan of an earlier start, and the listing of the folders that keys
  # it, relative to the card root, so that both mounts give one listing.
  [ -e $src/GRKIT/PLAN.BIN ] && { put $src/GRKIT/PLAN.BIN /tmp/gc/SCRIPT/GRKIT || return 1; }
  _dirs=
  for _d in GRKIT/EFFECTS SCRIPT/GRKIT/EFFECTS GRKIT/MODULES SCRIPT/GRKIT/MODULES; do [ -d $src/$_d ] && _dirs="$_dirs $_d"; done
  [ -n "$_dirs" ] && (cd $src && { echo $_dirs; ls -le $_dirs || ls -l $_dirs; }) >/tmp/gc/GRKIT/LIST.TXT 2>&1
  return 0
}
if mkdir /tmp/gc 2>/dev/null; then
  mount -t tmpfs -o size=16m tmpfs /tmp/gc
  log load $src
  # The deadline of the loader's mount: the measured time from the first
  # request to the camera's mount, less a margin.
  window=$(cat $src/SCRIPT/GRKIT/FAST.ON 2>/dev/null)
  [ "$window" -ge 0 ] 2>/dev/null || window=150
  # The request returns NG after about 5 s while the script runs on, so
  # the release stays well before that, while the loader can still remove
  # FAST.TRY when the request returns.
  [ $window -gt 400 ] && window=400
  now
  deadline=$((now + window - 50))
  resume=
  # The camera's try to mount the card and its mount, for the log.
  (
    tried=
    n=0
    while [ $n -lt 600 ]; do
      [ -z "$tried" ] && grep -q 'mnt /media/storage1' /var/log/messages && { tried=1; log camera-try; }
      grep -q storage1 /proc/mounts && { log camera-mount; break; }
      usleep 100000
      n=$((n + 1))
    done
  ) </dev/null >/dev/null 2>&1 &
  if ! fetch; then
    # Late: the rest waits for the camera's mount, as on the slow path.
    release
    mounted || exit 0
    resume=1
    fetch
  fi
  log copied
  if late; then
    release
    mounted || exit 0
  fi
  sh /tmp/gc/SCRIPT/GRKIT/SCAN.SH
  s=$?
  [ -e /tmp/gc/SCRIPT/GRKIT/PLAN.BAD ] && log plan-damaged
  if [ $s = 49 ]; then
    # A new plan: the effect folders, and the effects of GRKit 1.2 when
    # GRKIT has no library. Their copy is long, so it waits for the camera's
    # mount, in a process of its own: the request returns right after the
    # release, and the loader polls the mailbox for the plan.
    release
    (
      mounted || exit 0
      mkdir -p /tmp/gc/SCRIPT/GRKIT/EFFECTS
      [ -d $src/SCRIPT/GRKIT/EFFECTS ] && cp -r $src/SCRIPT/GRKIT/EFFECTS /tmp/gc/SCRIPT/GRKIT/
      [ -d $src/GRKIT/EFFECTS ] && cp -r $src/GRKIT/EFFECTS /tmp/gc/GRKIT/
      if [ ! -e $src/GRKIT/STATE.TXT ] || [ ! -d $src/GRKIT/EFFECTS ]; then
        [ -d $src/SCRIPT/PLUGINS/EFFECTS ] && mkdir -p /tmp/gc/SCRIPT/PLUGINS && cp -r $src/SCRIPT/PLUGINS/EFFECTS /tmp/gc/SCRIPT/PLUGINS/
      fi
      log effects
      sh /tmp/gc/SCRIPT/GRKIT/SCAN.SH
    ) </dev/null >/dev/null 2>&1 &
  fi
fi
release
exit 0
