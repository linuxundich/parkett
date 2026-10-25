#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Queries Parkett via AT-SPI (in the test VM, for smoke.sh).

  atspi.py tabs              tab names, one per line
  atspi.py focused           role and name of the focused element
  atspi.py texts             last lines with text per terminal (panes in tree order)
  atspi.py press NAME [ROLE] first action of the first element with this name
                             (ROLE with _ instead of spaces, e.g. toggle_button)
"""
import sys

import gi

gi.require_version("Atspi", "2.0")
from gi.repository import Atspi  # noqa: E402


def nodes():
    desktop = Atspi.get_desktop(0)
    out = []

    def walk(o):
        try:
            out.append(o)
            for i in range(o.get_child_count()):
                c = o.get_child_at_index(i)
                if c:
                    walk(c)
        except Exception:  # elements vanish during the walk
            pass

    for i in range(desktop.get_child_count()):
        app = desktop.get_child_at_index(i)
        if app and app.get_name() == "parkett":
            walk(app)
    return out


def main():
    cmd = sys.argv[1] if len(sys.argv) > 1 else ""
    if cmd == "tabs":
        # Pages of the tab bar: the name starts with the number ("1 bash · ~")
        for o in nodes():
            name = o.get_name() or ""
            if o.get_role_name() == "panel" and name[:1].isdigit() and " " in name:
                print(name)
    elif cmd == "focused":
        for o in nodes():
            if o.get_state_set().contains(Atspi.StateType.FOCUSED):
                print(f"{o.get_role_name()}: {o.get_name() or ''}")
    elif cmd == "texts":
        for o in nodes():
            if o.get_role_name() == "terminal":
                text = Atspi.Text.get_text(o, 0, Atspi.Text.get_character_count(o))
                lines = [line for line in text.splitlines() if line.strip()]
                print(" | ".join(lines[-4:]))
    elif cmd == "press":
        name = sys.argv[2]
        role = sys.argv[3].replace("_", " ") if len(sys.argv) > 3 else None
        for o in nodes():
            if o.get_name() == name and (role is None or o.get_role_name() == role):
                Atspi.Action.do_action(o, 0)
                return
        sys.exit(f"not found: {name}")
    else:
        sys.exit(__doc__)


main()
