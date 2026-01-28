#!/usr/bin/env python
import sys
from base64 import standard_b64encode

first, eof, buf = True, False, memoryview(bytearray(3 * 4096 // 4))
w = sys.stdout.buffer.write
with open(sys.argv[-1], 'rb') as f:
    while not eof:
        p = buf[:]
        while p and not eof:
            n = f.readinto1(p)
            p, eof = p[n:], n == 0
        encoded = standard_b64encode(buf[:len(buf)-len(p)])
        metadata, first = "a=T,f=100," if first else "", False
        w(f'\x1b_G{metadata}m={0 if eof else 1};'.encode('ascii'))
        w(encoded)
        w(b'\x1b\\')
