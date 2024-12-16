#!/usr/bin/env python
import argparse
from random import randint

'''
Simple Linear Congruential Generator.
'''
def lcg(m: int, a:int, c:int, x:int):
    while True:
        x = (a * x + c) % m
        yield x

'''
Copy of the Windows rand func, is truncated because Windows
'''
def rand():
    return (next(win_rand) >> 16) & 0x7FFF

'''
Returns full "Windows" VC++ rand value
'''
def extended_rand():
    return next(win_rand)

def seed(seed):
    global win_rand
    win_rand = lcg(win_m, win_a, win_c, seed)

win_rand = None
win_m = 2**32
win_a = 214013
win_c = 2531011

if __name__ == '__main__':    
    parser = argparse.ArgumentParser()

    parser.add_argument("-s", "--seed", type=int, default=randint(0, 2**32), help="RNG seed.")

    parser.add_argument("-n", "--number", type=int, default=1, help="Number of random values to generate.")

    args = parser.parse_args()

    seed(args.seed & 0xFFFFFFFF)

    print(f"Generating random values with seed {args.seed & 0xFFFFFFFF}...")
    rand_str = ""
    for i in range(0, args.number):
        rand_val = rand()
        rand_str = rand_str + f"{rand_val} "
    print(f"Generated {args.number} item(s):")
    print(rand_str)
