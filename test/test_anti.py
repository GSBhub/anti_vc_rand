#!/usr/bin/env python
from lib.anti_rand import anti_win_rand
from lib.windows_rand import seed, rand
import pytest
from random import randint

def _do_test_helper(num_iterations, rand_seed):
    rand_values = []
    seed(rand_seed)
    for i in range(0, num_iterations):
        rand_values.append(rand())
    return anti_win_rand(rand_values, num_iterations)

def _do_test_helper_2(num_iterations, num_samples, rand_seed):
    rand_values = []
    seed(rand_seed)
    for i in range(0, num_iterations):
        rand_values.append(rand())
    return anti_win_rand(rand_values[num_samples:], num_iterations)

def test_one_antirand():
    rand_seed = randint(0, 2**32) & 0xFFFFFFFF
    seeds = _do_test_helper(1, rand_seed)
    assert rand_seed in seeds

def test_three_antirand():
    rand_seed = randint(0, 2**32) & 0xFFFFFFFF
    seeds = _do_test_helper(3, rand_seed)
    assert rand_seed in seeds
    assert len(seeds) < 5

def test_ten_antirand():
    rand_seed = randint(0, 2**32) & 0xFFFFFFFF
    seeds = _do_test_helper(10, rand_seed)
    assert rand_seed in seeds
    assert len(seeds) <= 5

def test_ten_thousand_antirand():
    rand_seed = randint(0, 2**32) & 0xFFFFFFFF
    seeds = _do_test_helper(10000, rand_seed)
    assert rand_seed in seeds
    assert len(seeds) <= 3

def test_ten_antirand_var_2():
    rand_seed = randint(0, 2**32) & 0xFFFFFFFF
    seeds = _do_test_helper_2(10, randint(0, 9), rand_seed)
    assert rand_seed in seeds
    assert len(seeds) <= 5

def test_ten_thousand_antirand_var_2():
    rand_seed = randint(0, 2**32) & 0xFFFFFFFF
    seeds = _do_test_helper_2(10000, randint(0, 10000-15), rand_seed)
    assert rand_seed in seeds
    assert len(seeds) <= 3
