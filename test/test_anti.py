#!/usr/bin/env python
"""Checks the Rust `anti_lcg` module against the pure-Python reference in lib/windows_rand.py."""
from random import randint

import pytest

import anti_lcg
from lib.windows_rand import seed, rand


def _rand_values(rand_seed, num_iterations):
    seed(rand_seed)
    return [rand() for _ in range(num_iterations)]


def _do_test_helper(num_iterations, rand_seed):
    return anti_lcg.anti_win_rand(_rand_values(rand_seed, num_iterations), num_iterations)


def _do_test_helper_2(num_iterations, num_samples, rand_seed):
    rand_values = _rand_values(rand_seed, num_iterations)
    return anti_lcg.anti_win_rand(rand_values[num_samples:], num_iterations)


def _random_seed():
    return randint(0, 2**32) & 0xFFFFFFFF


def test_one_antirand():
    rand_seed = _random_seed()
    seeds = _do_test_helper(1, rand_seed)
    assert rand_seed in seeds


@pytest.mark.parametrize("n", [3, 10, 10000])
def test_antirand(n):
    rand_seed = _random_seed()
    seeds = _do_test_helper(n, rand_seed)
    assert rand_seed in seeds
    # Bit 31 never reaches the output, so s and s ^ 2^31 are indistinguishable.
    assert rand_seed ^ 0x80000000 in seeds
    assert len(seeds) <= 4


def test_ten_antirand_var_2():
    rand_seed = _random_seed()
    seeds = _do_test_helper_2(10, randint(0, 9), rand_seed)
    assert rand_seed in seeds
    assert len(seeds) <= 4 or len(seeds) == 2**16 * 2  # a single value leaves 2^16 classes


def test_ten_thousand_antirand_var_2():
    rand_seed = _random_seed()
    seeds = _do_test_helper_2(10000, randint(0, 10000 - 15), rand_seed)
    assert rand_seed in seeds
    assert len(seeds) <= 4


def test_generate_matches_reference():
    rand_seed = _random_seed()
    assert anti_lcg.generate(rand_seed, 100) == _rand_values(rand_seed, 100)


def test_scattered_samples():
    rand_seed = _random_seed()
    values = _rand_values(rand_seed, 5000)
    samples = [(i, values[i - 1]) for i in (17, 900, 2500, 4999)]
    assert rand_seed in anti_lcg.recover_seed(samples=samples)


def test_other_preset_roundtrip():
    rand_seed = randint(0, 2**48 - 1)
    values = anti_lcg.generate(rand_seed, 3, skip=997, preset="java")
    assert rand_seed in anti_lcg.recover_seed(values, 1000, preset="java")


def test_recover_params():
    states = anti_lcg.generate(12345, 10, states=True, a=1103515245, c=12345, m=2**31)
    assert (1103515245, 12345, 2**31) in anti_lcg.recover_params(states)
