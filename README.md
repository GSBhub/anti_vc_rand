# Anti-Windows Rand
Example script for reversing a Windows seed given enough random values from the rand() function in vc++.

## Theory
`rand` in VC++ uses a Linear Congruential Generator with the constants `a=214013`, `c=2531011`, and `m=2^32` to generate random numbers. LCG basically goes `x = (a * x + c) % m`, where the first `x` value is the `seed` for the RNG, and every subsequent value is the previous output of `rand`. 


In a standard LCG, the modulus operation can be undone using the Extended Euclidian Algorithm, which allows you to "replay" the LCG function backwards if given a specific random value. VC++'s `rand` is a bit different, in that it only returns some of the bytes output by `rand`. Windows `rand` shifts the output of the previous LCG right by 16 bits, then ands it with the constant `0x7FFF` (hence the `RAND_MAX` value being 0x7FFF in Windows)


## Reversing the "seed" given some random samples
If given enough (generally 3 or more) random samples, and some knowledge of how many previous random samples were generated with a seed, one can attempt to "brute force" the seed.

The algorithm works like this:
    1. For the last given random value, shift the `rand` value left by 16 bits. (ex, `rand()` gives `0xDEAD` -> expanded rand of `0xDEAD0000`

    2. For all values in 0, 0x1FFFF, add it to the base expanded rand and use it to generate an anti-lcg.

    3. For all anti-lcg generators, generate the next LCG (so the previous full rand value in the chain). Run it through the Windows bit truncate process (>> 16 & 0x7FFF), compare to the next known random.

    4. If they are not equal, discard that generator. 

    5. If they are equal, save that generator as a "likely" candidate.

    6. Repeat 3 - 5 for the number of "guesses", that is, the number of random numbers generated with your target seed given your initial guess. You don't need every single random number generated between your starting sample and the final, you just need a few samples (ie, 3-5 of the last numbers generated in a 10000 number sequence can recover the entire sequence).

    Once you have iterated through all potential guesses, your next iteration for the remaining should be the "seed" value.

## Current bugs
The test cases sometimes randomly fails, which indicates a flaw in my algorithm. I kind of just threw this together in a weekend, so my thought is that the `range (0, 0x1ffff)` loop doesn't fully undo the `val >> 16 & 0x7FFF` that windows does in standard calculations.
