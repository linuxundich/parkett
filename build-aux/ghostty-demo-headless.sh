#!/bin/bash
# Run a command in a private headless session (mutter --headless, own D-Bus/runtime dir):
#   dbus-run-session -- build-aux/ghostty-demo-headless.sh target/debug/examples/demo
# Private headless session: own runtime dir and D-Bus, nothing of the desktop session is touched.
set -e
RT=$(mktemp -d); chmod 700 $RT
export XDG_RUNTIME_DIR=$RT HOME_BAK=$HOME GTK_A11Y=none GDK_DEBUG=no-portals GSK_RENDERER=cairo GDK_BACKEND=wayland
export XDG_CONFIG_HOME=$RT/cfg XDG_DATA_HOME=$RT/data XDG_CACHE_HOME=$RT/cache
unset WAYLAND_DISPLAY DISPLAY
mutter --headless --no-x11 --virtual-monitor 1280x720 --wayland-display wayland-hl >$RT/mutter.log 2>&1 &
MPID=$!
for i in $(seq 50); do [ -S $RT/wayland-hl ] && break; sleep 0.1; done
export WAYLAND_DISPLAY=wayland-hl
"$@" ; RC=$?
kill $MPID 2>/dev/null; wait $MPID 2>/dev/null
rm -rf $RT
exit $RC
