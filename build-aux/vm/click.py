#!/usr/bin/env python3
"""Mausklick in der Test-VM per QMP (absolute Koordinaten in Bildschirmpixeln).
click.py X Y [BREITE HÖHE]   -> Linksklick; Standardauflösung 1280x800
"""
import json, subprocess, sys, time
DOM = "fedora44-wor"
x, y = int(sys.argv[1]), int(sys.argv[2])
w, h = (int(sys.argv[3]), int(sys.argv[4])) if len(sys.argv) > 4 else (1280, 800)
def qmp(events):
    subprocess.run(["virsh", "-c", "qemu:///session", "qemu-monitor-command", DOM,
                    json.dumps({"execute": "input-send-event", "arguments": {"events": events}})],
                   check=True, capture_output=True)
qmp([{"type": "abs", "data": {"axis": "x", "value": x * 32767 // (w - 1)}},
     {"type": "abs", "data": {"axis": "y", "value": y * 32767 // (h - 1)}}])
time.sleep(0.1)
for down in (True, False):
    qmp([{"type": "btn", "data": {"down": down, "button": "left"}}])
    time.sleep(0.05)
