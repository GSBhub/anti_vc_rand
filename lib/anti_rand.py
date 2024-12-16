from lib.extended_euclid import *
from lib.windows_rand import *

# Run "next" on all generators and return the generators
# that produce a windows rand value equal to the "guess"
def search_for_guess(guess, generators):
    candidates = []
    for generator in generators:
        expanded_guess = next(generator)
        if guess == ((expanded_guess >> 16) & 0x7FFF):
            candidates.append(generator)

    return candidates

'''
Attempt to recover the seed from windows rand
'''
def anti_win_rand(guesses, num_rand, m=win_m, a=win_a, c=win_c):
    # first seed for anti_lcg should be based on the last value of "guesses".
    # will need to iterate through all possible values 
    # algorithm:
    #   extend the random number out to brute-force all potential "possible" options
    #   run anti-lcg for each
    #   convert back to the truncated value and compare to the "next" value
    #   keep going num_rand times until
    seeds = []
    sequences = {}
    attempts = num_rand

    starting_value = guesses[-1] << 16 # shift the random number over, producing the higher-order random number
    guesses = guesses[:-1] # pop off the last item
    
    generators = []
    # generate generators for all anti-lcg values possible
    for i in range(0, 0x10000):
        generators.append(anti_lcg(m, a, c, starting_value + i))

    # also add the MSB carved off by the and of 0x7FFF
    for j in range(0, 0x10000):
        generators.append(anti_lcg(m, a, c, (starting_value + j) | 0x80000000))
    
    print(f"Starting search with {len(generators)} generators...")

    while (attempts > 1) and (len(generators) > 1):
        if len(guesses) >= 1:
            next_value = guesses[-1]
            guesses = guesses[:-1]
            # calculate all possible LCG values that could produce next value and trim the list down
            generators = search_for_guess(next_value, generators)
            print(f"Pass {num_rand - attempts + 1}: {len(generators)} generators remain.")
        else:
            # at this point, we've exhausted all our guesses, so we just run through each
            # remaining generator until attempts are exhausted 
            for generator in generators:
                next(generator)

        attempts -= 1
    
    for generator in generators:
        seed_candidate = next(generator)
        seeds.append(seed_candidate)

    return seeds

