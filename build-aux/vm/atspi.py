#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Fragt Parkett über AT-SPI ab (in der Test-VM, für smoke.sh).

  atspi.py tabs              Reiter-Namen, einer je Zeile
  atspi.py focused           Rolle und Name des Elements mit Fokus
  atspi.py press NAME [ROLE] erste Aktion des ersten Elements mit diesem Namen
                             (ROLE mit _ statt Leerzeichen, z. B. toggle_button)
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
        except Exception:  # Elemente verschwinden während des Durchlaufs
            pass

    for i in range(desktop.get_child_count()):
        app = desktop.get_child_at_index(i)
        if app and app.get_name() == "parkett":
            walk(app)
    return out


def main():
    cmd = sys.argv[1] if len(sys.argv) > 1 else ""
    if cmd == "tabs":
        # Seiten der Reiterleiste: Name beginnt mit der Nummer („1 bash · ~“)
        for o in nodes():
            name = o.get_name() or ""
            if o.get_role_name() == "panel" and name[:1].isdigit() and " " in name:
                print(name)
    elif cmd == "focused":
        for o in nodes():
            if o.get_state_set().contains(Atspi.StateType.FOCUSED):
                print(f"{o.get_role_name()}: {o.get_name() or ''}")
    elif cmd == "press":
        name = sys.argv[2]
        role = sys.argv[3].replace("_", " ") if len(sys.argv) > 3 else None
        for o in nodes():
            if o.get_name() == name and (role is None or o.get_role_name() == role):
                Atspi.Action.do_action(o, 0)
                return
        sys.exit(f"nicht gefunden: {name}")
    else:
        sys.exit(__doc__)


main()
