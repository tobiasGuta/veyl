#!/usr/bin/env python3
"""Install or remove the Veyl binary, desktop entry and icon for this user."""

import argparse
import os
from pathlib import Path
import shutil
from tempfile import TemporaryDirectory


APP_ID = "io.github.tobiasGuta.Veyl"
ROOT = Path(__file__).resolve().parent.parent
ICON = Path("icons/hicolor/scalable/apps") / f"{APP_ID}.svg"


def desktop_string(value):
    return (value.replace("\\", "\\\\").replace("\n", "\\n")
            .replace("\t", "\\t").replace("\r", "\\r"))


def desktop_exec(path):
    quoted = "".join(
        "\\" + char if char in '\\"$`' else char for char in str(path)
    ).replace("%", "%%")
    return desktop_string(f'"{quoted}"')


def install(binary, executable, icon, launcher):
    source_icon = ROOT / "resources" / ICON
    template = ROOT / "resources" / launcher.name
    if not binary.is_file() or not os.access(binary, os.X_OK):
        raise ValueError(f"{binary} is not executable. Run cargo build --locked --release first.")

    desktop = "\n".join(
        f"Exec={desktop_exec(executable)}" if line.startswith("Exec=")
        else f"Icon={desktop_string(str(icon))}" if line.startswith("Icon=")
        else line
        for line in template.read_text(encoding="utf-8").splitlines()
    ) + "\n"

    for source, target, mode in [
        (binary, executable, 0o755),
        (source_icon, icon, 0o644),
        (None, launcher, 0o644),
    ]:
        target.parent.mkdir(parents=True, exist_ok=True)
        with TemporaryDirectory(prefix=".veyl-install-", dir=target.parent) as staging:
            staged = Path(staging) / target.name
            if source is None:
                staged.write_text(desktop, encoding="utf-8")
            else:
                shutil.copyfile(source, staged)
            staged.chmod(mode)
            staged.replace(target)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=["install", "uninstall"])
    parser.add_argument("--binary", type=Path, default=ROOT / "target/release/veyl",
                        help="path to an already-built Veyl executable")
    args = parser.parse_args()
    data = Path(os.environ.get("XDG_DATA_HOME") or Path.home() / ".local/share")
    if not data.is_absolute():
        parser.error("XDG_DATA_HOME must be an absolute path")
    executable = Path.home() / ".local/bin/veyl"
    icon = data / ICON
    launcher = data / "applications" / f"{APP_ID}.desktop"

    try:
        if args.action == "install":
            install(args.binary, executable, icon, launcher)
            print("Installed Veyl. Open Veyl from the application launcher.")
        else:
            for path in (launcher, icon, executable):
                path.unlink(missing_ok=True)
            print("Removed the Veyl executable, launcher, and icon. User settings were kept.")
    except (OSError, ValueError) as error:
        parser.exit(1, f"Veyl: {error}\n")


if __name__ == "__main__":
    main()
