//! Small number-theory helpers (port of `lib/extended_euclid.py`).

/// Extended Euclid: returns `(g, x, y)` with `a*x + b*y == g == gcd(a, b)`.
pub fn egcd(a: i128, b: i128) -> (i128, i128, i128) {
    let (mut old_r, mut r) = (a, b);
    let (mut old_x, mut x) = (1i128, 0i128);
    let (mut old_y, mut y) = (0i128, 1i128);
    while r != 0 {
        let q = old_r / r;
        (old_r, r) = (r, old_r - q * r);
        (old_x, x) = (x, old_x - q * x);
        (old_y, y) = (y, old_y - q * y);
    }
    (old_r, old_x, old_y)
}

pub fn gcd(a: u128, b: u128) -> u128 {
    let (mut a, mut b) = (a, b);
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

/// Inverse of `a` modulo `m` (`m <= 2^64`), or `None` if `gcd(a, m) != 1`.
pub fn mod_inverse(a: u128, m: u128) -> Option<u128> {
    if m == 1 {
        return Some(0);
    }
    let (g, x, _) = egcd((a % m) as i128, m as i128);
    if g != 1 {
        return None;
    }
    Some(x.rem_euclid(m as i128) as u128)
}

/// `(a * b) % m` for operands below `m <= 2^64`.
#[inline(always)]
pub fn mul_mod(a: u128, b: u128, m: u128) -> u128 {
    // a, b < 2^64 so the product fits in u128.
    (a * b) % m
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inverse_roundtrip() {
        let m = 1u128 << 32;
        let inv = mod_inverse(214013, m).unwrap();
        assert_eq!(mul_mod(inv, 214013, m), 1);
        assert_eq!(mod_inverse(4, m), None);
        let p = (1u128 << 31) - 1;
        let inv = mod_inverse(16807, p).unwrap();
        assert_eq!(mul_mod(inv, 16807, p), 1);
        let m64 = 1u128 << 64;
        let inv = mod_inverse(6364136223846793005, m64).unwrap();
        assert_eq!((inv * 6364136223846793005) % m64, 1);
    }
}
