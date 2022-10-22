#!/usr/bin/python3

import sys
import random

try:
    (_file, amount) = sys.argv
except ValueError:
    sys.exit("usage: mkdata.py [AMOUNT]")

print("side,qty,price")
for _ in range(0, int(amount)):
    side = random.choice(["buy", "sell"])
    qty = random.randrange(1, 100)
    price = random.randrange(100, 150, step=1)
    print(f"{side},{qty},{price}")
