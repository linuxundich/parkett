#!/usr/bin/env python3
"""Mouse click in the test VM via QMP (absolute coordinates in screen pixels).
click.py X Y [WIDTH HEIGHT]  -> left click; without them the VM's current resolution applies
CLICKS=2 click.py X Y        -> double click
BUTTON=right click.py X Y    -> right click (also middle)
"""
import json, subprocess, sys, time
DOM = "fedora44-wor"
x, y = int(sys.argv[1]), int(sys.argv[2])
if len(sys.argv) > 4:
    w, h = int(sys.argv[3]), int(sys.argv[4])
else:  # current resolution from a screenshot
    import tempfile
    with tempfile.NamedTemporaryFile(suffix=".img") as f:
        subprocess.run(["virsh", "-c", "qemu:///session", "screenshot", DOM, f.name], check=True, capture_output=True)
        dims = subprocess.run(["magick", "identify", "-format", "%w %h", f.name], check=True, capture_output=True, text=True).stdout
    w, h = map(int, dims.split())
def qmp(events):
    subprocess.run(["virsh", "-c", "qemu:///session", "qemu-monitor-command", DOM,
                    json.dumps({"execute": "input-send-event", "arguments": {"events": events}})],
                   check=True, capture_output=True)
qmp([{"type": "abs", "data": {"axis": "x", "value": x * 32767 // (w - 1)}},
     {"type": "abs", "data": {"axis": "y", "value": y * 32767 // (h - 1)}}])
time.sleep(0.1)
import os
for _ in range(int(os.environ.get("CLICKS", "1"))):
    for down in (True, False):
        qmp([{"type": "btn", "data": {"down": down, "button": os.environ.get("BUTTON", "left")}}])
        time.sleep(0.04)
