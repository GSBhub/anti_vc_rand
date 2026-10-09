#!/usr/bin/env python
import argparse

from anti_lcg import recover_seed

if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument("-r", "--random_values", nargs='+', help="List of random values for guess.", type=int, required=True)
    parser.add_argument("-n", "--n_guesses", type=int, help="Number of rand() calls since seeding (the last value's call number). Defaults to the number of values.")
    parser.add_argument("-p", "--preset", default="msvc", help="Generator preset (see anti_lcg.presets()).")
    args = parser.parse_args()

    seeds = recover_seed(args.random_values, args.n_guesses, preset=args.preset)
    print(f"Got {len(seeds)} candidates for seeds for the provided list of random values.")
    print(seeds)
