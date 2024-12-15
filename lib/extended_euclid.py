# extended euclid algorithm implementation
def extended_euclid(a, b):
    if a == 0:
        return (b, 0, 1)

    gcd, x1, y1 = extended_euclid(b % a, a)
    x = y1 - (b // a) * x1
    y = x1

    return gcd, x, y

'''
Given a, m, calculate inverse mod of a and m
'''
def mod_inverse(a, m):
    gcd, x, y = extended_euclid(a, m)
    if gcd != 1:
        print(f"No GCD for {a} % {m}")
        return None
    else:
        return x % m

'''
Given LCG params, runs LCG algorithm in reverse for provided X.
'''
def anti_lcg(m: int, a: int, c: int, x: int):
    antimod_a_m = mod_inverse(a, m)
    while True:
        x = antimod_a_m * (x - c) % m
        yield x
