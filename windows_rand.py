#!/usr/bin/env python
import argparse

def lcg(m: int, a:int, c:int, x:int):
    while True:
        x = (a * x + c) % m
        yield x

def vc_rand():
    return (next(win_rand) >> 16) & 0x7FFF

def rand():
    return next(win_rand)

def extended_gcd(a, b):
    if a == 0:
        return (b, 0, 1)

    gcd, x1, y1 = extended_gcd(b % a, a)
    x = y1 - (b // a) * x1
    y = x1

    return gcd, x, y

def mod_inverse(a, m):
    gcd, x, y = extended_gcd(a, m)
    if gcd != 1:
        print(f"No GCD for {a} % {m}")
        return None
    else:
        return x % m

def anti_lcg(m: int, a: int, c: int, x: int):
    while True:
        x = mod_inverse(a, m) * (x - c) % m
        yield x


# Run "next" on all generators and return the generators
# that produce a windows rand value equal to the "guess"
def search_for_guess(guess, generators):
    candidates = []
    for generator in generators:
        expanded_guess = next(generator)
        if guess == ((expanded_guess >> 16) & 0x7FFF):
            candidates.append(generator)

    print(f"Found {len(candidates)} potential values that could generate {guess}")
    return candidates

'''
Attempt to recover the seed from windows rand
'''
def anti_win_rand(m, a, c, guesses, num_rand):
    # first seed for anti_lcg should be based on the last value of "guesses".
    # will need to iterate through all possible values 
    # algorithm:
    #   extend the random number out to brute-force all potential "possible" options
    #   run anti-lcg for each
    #   convert back to the truncated value and compare to the "next" value
    #   keep going num_rand times until
    attempts = num_rand
    
    starting_value = guesses[-1] << 16 # shift the random number over, producing the higher-order random number
    guesses = guesses[:-1] # pop off the last item
    
    generators = []
    # generate generators for all anti-lcg values possible
    for i in range(0, 0x1ffff):
        generators.append(anti_lcg(m, a, c, starting_value + i))
    
    while attempts > 1:
        
        if len(guesses):
            next_value = guesses[-1]
            guesses = guesses[:-1]
            # calculate all possible LCG values that could produce next value and trim the list down
            generators = search_for_guess(next_value, generators)

        else:
            # at this point, we've exhausted all our guesses, so we just run through each
            # remaining generator until attempts are exhausted
            for generator in generators:
                next(generator)

        attempts -= 1

    print("Found the following potential seeds from the provided guest list:")
    for generator in generators:
        print(f"Seed: {next(generator)}")


if __name__ == '__main__':

    win_m = 2**32
    win_a = 214013
    win_c = 2531011
    seed = 12345678 # just using a simple seed for now

    random_guesses = []

    win_rand = lcg(win_m, win_a, win_c, seed)
    #rand_1 = rand()
    #rand_2 = rand()
    #rand_3 = rand()
    rand_1 = vc_rand()
    rand_2 = vc_rand()
    rand_3 = vc_rand()

    print(f"Generated 3 random numbers using seed {hex(seed)}: {hex(rand_1)} {hex(rand_2)} {hex(rand_3)}")

    guesses = [rand_3]
    print(f"Attempting to guess seeds with only rand_3...")
    anti_win_rand(win_m, win_a, win_c, guesses, 3)

    guesses = [rand_2, rand_3]
    print(f"Attempting to guess seeds with rand_3 and rand_2...")
    anti_win_rand(win_m, win_a, win_c, guesses, 3)

    guesses = [rand_1, rand_2, rand_3]
    print(f"Attempting to guess seeds with all 3 rand values...")
    anti_win_rand(win_m, win_a, win_c, guesses, 3)

    #anti_win_rand = anti_lcg(win_m, win_a, win_c, rand_3)

    #print(f"rand_2 vs anti: {hex(rand_2)} vs {hex(next(anti_win_rand))}")
    #print(f"rand_1 vs anti: {hex(rand_1)} vs {hex(next(anti_win_rand))}")
    #print(f"seed vs anti: {hex(seed)} vs {hex(next(anti_win_rand))}")

