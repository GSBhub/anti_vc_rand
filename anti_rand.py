#!/usr/bin/env python
from lib.anti_rand import anti_win_rand
import argparse

if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument("-r", "--random_values", nargs='+', help="List of random values for guess.", type=int, required=True)
    parser.add_argument("-n", "--n_guesses", type=int, help="Approximate number of random values back to guess for the seed")
    args = parser.parse_args()

    print(args.random_values)
    print(args.n_guesses)
    seeds = anti_win_rand(args.random_values, args.n_guesses)
    print (f"Got {len(seeds)} candidates for seeds for the provided list of random values.")
    print(seeds)
