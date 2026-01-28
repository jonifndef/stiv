#!/usr/bin/env python3
import sys
import base64
from PIL import Image

def tmux_wrap(payload: bytes) -> bytes:
    return b"\x1bPtmux;" + payload.replace(b"\x1b", b"\x1b\x1b") + b"\x1b\\"

def main(path):
    img = Image.open(path).convert("RGB")
    width, height = img.size
    raw = img.tobytes()
    encoded = base64.b64encode(raw)
    CHUNK = 4096

    for i in range(0, len(encoded), CHUNK):
        chunk = encoded[i:i+CHUNK]
        m = 1 if i + CHUNK < len(encoded) else 0

        kitty_msg = (
            f"\x1b_G{'a=T,' if i == 0 else ''}"
            f"f=24,s={width},v={height},m={m};"
        ).encode("ascii") + chunk + b"\x1b\\"

        #sys.stdout.buffer.write(tmux_wrap(kitty_msg))
        sys.stdout.buffer.write(kitty_msg)

if __name__ == "__main__":
    if len(sys.argv) != 2:
        print("usage: kitty_raw.py <image>", file=sys.stderr)
        sys.exit(1)
    main(sys.argv[1])
