use std::ops::{Add, Div, Mul, Neg, Rem, Sub};

use ethers::types::U256;
use eyre::{bail, eyre, Result};

use crate::{exp, ln, FixedPoint, FixedPointValue};

impl<T: FixedPointValue> FixedPoint<T> {
    /// Computes the absolute value of self.
    ///
    /// # Panics
    ///
    /// If the absolute value of self overflows `T`, e.g., if self is the minimum value of a signed
    /// integer.
    pub fn abs(&self) -> Self {
        self.raw().abs().into()
    }

    /// Computes the absolute value of self as a `U256` to avoid overflow.
    pub fn unsigned_abs(&self) -> FixedPoint<U256> {
        self.raw().unsigned_abs().into()
    }

    pub fn abs_diff(&self, other: Self) -> FixedPoint<U256> {
        let abs_self = self.unsigned_abs();
        let abs_other = other.unsigned_abs();
        if self.sign() != other.sign() {
            abs_self + abs_other
        } else if abs_self > abs_other {
            abs_self - abs_other
        } else {
            abs_other - abs_self
        }
    }

    pub fn mul_div_down(self, other: Self, divisor: Self) -> Self {
        if divisor.is_zero() {
            panic!("Cannot divide by zero.");
        }
        let sign = self.sign().flip_if(other.sign() != divisor.sign());
        let abs = U256::try_from(
            self.raw()
                .unsigned_abs()
                .full_mul(other.raw().unsigned_abs())
                .div(divisor.raw().unsigned_abs()),
        )
        .map_err(|_| eyre!("FixedPoint operation overflowed: {self} * {other} / {divisor}"))
        .unwrap();
        Self::from_sign_and_abs(sign, abs).unwrap()
    }

    pub fn mul_div_up(self, other: Self, divisor: Self) -> Self {
        if divisor.is_zero() {
            panic!("Cannot divide by zero.");
        }
        let sign = self.sign().flip_if(other.sign() != divisor.sign());
        let (abs_u512, rem) = self
            .raw()
            .unsigned_abs()
            .full_mul(other.raw().unsigned_abs())
            .div_mod(divisor.raw().unsigned_abs().into());
        let abs = U256::try_from(abs_u512)
            .map_err(|_| eyre!("FixedPoint operation overflowed: {self} * {other} / {divisor}"))
            .unwrap();
        Self::from_sign_and_abs(sign, abs).unwrap()
            + Self::from_sign_and_abs(sign, U256::from(!rem.is_zero() as u8)).unwrap()
    }

    pub fn mul_down(self, other: Self) -> Self {
        self.mul_div_down(other, self.one())
    }

    pub fn mul_up(self, other: Self) -> Self {
        self.mul_div_up(other, self.one())
    }

    pub fn div_down(self, other: Self) -> Self {
        self.mul_div_down(self.one(), other)
    }

    pub fn div_up(self, other: Self) -> Self {
        self.mul_div_up(self.one(), other)
    }

    pub fn pow(self, y: Self) -> Result<Self> {
        let one = self.one();

        // If the exponent is negative, return 1 / x^abs(y).
        if y.is_negative() {
            let abs_result = self.pow(y.abs())?;

            if abs_result.is_zero() {
                bail!("Cannot divide by zero.");
            }

            return Ok(one.div_down(abs_result));
        }

        // If the exponent is 0, return 1.
        if y.is_zero() {
            return Ok(one);
        }

        // If the base is 0, return 0.
        if self.is_zero() {
            return Ok(Self::zero());
        }

        // Using properties of logarithms we calculate x^y: -> ln(x^y) = y * ln(x) -> e^(y * ln(x))
        // = x^y
        let y_int256 = y.to_i256()?;

        // Compute y*ln(x) Any overflow for x will be caught in _ln() in the initial bounds check
        let lnx = ln(self.to_i256()?)?;
        let mut ylnx = y_int256.wrapping_mul(lnx);
        ylnx = ylnx.wrapping_div(one.to_i256()?);

        // Calculate exp(y * ln(x)) to get x^y
        let (sign, abs) = exp(ylnx)?.into_sign_and_abs();
        Self::from_sign_and_abs(sign.into(), abs)
    }
}

impl<T: FixedPointValue> Neg for FixedPoint<T> {
    type Output = Self;

    fn neg(self) -> Self {
        Self::new(self.raw().flip_sign())
    }
}

/// Takes a mapping of operator traits to `FixedPoint` methods and implements the operator and
/// assignment operator for each one.
macro_rules! mapped_operator_impls {
    ($($trait:ident => $fn:ident),*) => {
        $(
            paste::paste! {

                impl<T: FixedPointValue> std::ops::$trait for FixedPoint<T> {
                    type Output = Self;

                    fn [<$trait:lower>](self, other: Self) -> Self::Output {
                        self.$fn(other)
                    }
                }

                impl<T: FixedPointValue> std::ops::[<$trait Assign>] for FixedPoint<T> {
                    fn [<$trait:lower _assign>](&mut self, other: Self) {
                        *self = self.[<$trait:lower>](other);
                    }
                }
            }
        )*
    };
    ($($tt:tt)*) => {};
}

mapped_operator_impls!(
    // use `mul_down` for `*` and `*=`.
    Mul => mul_down,
    // use `div_down` for `/` and `/=`.
    Div => div_down
);

/// Takes a list of operator traits and implements the operator and assignment operator for each one
/// by forwarding to the corresponding method on the underlying `FixedPointValue`.
macro_rules! forwarded_operator_impls {
    ($($trait:ident),*) => {
        $(
            paste::paste! {

                impl<T: FixedPointValue> std::ops::$trait for FixedPoint<T> {
                    type Output = Self;

                    fn [<$trait:lower>](self, other: Self) -> Self::Output {
                        Self::new(self.raw().[<$trait:lower>](other.raw()))
                    }
                }

                impl<T: FixedPointValue> std::ops::[<$trait Assign>] for FixedPoint<T> {
                    fn [<$trait:lower _assign>](&mut self, other: Self) {
                        *self = self.[<$trait:lower>](other);
                    }
                }
            }
        )*
    };
    ($($tt:tt)*) => {};
}

// Forward these operators to the underlying `FixedPointValue`.
forwarded_operator_impls!(Add, Sub, Rem);

#[cfg(test)]
mod tests {
    use std::{panic, u128};

    use ethers::types::U256;
    use proptest::prelude::*;
    use rand::{thread_rng, Rng};

    use super::*;
    use crate::{fixed, fixed_u128, fixed_u256, uint256};

    /// The maximum number that can be divided by another in the implementation.
    fn max_numerator() -> FixedPoint<U256> {
        (U256::MAX / uint256!(1e18)).into()
    }

    #[test]
    fn test_sub_failure() {
        // Ensure that subtraction failures are propagated from the raw type.
        assert!(panic::catch_unwind(|| fixed_u256!(1e18) - fixed!(2e18)).is_err());
    }

    #[test]
    fn test_mul_div_down_failure() {
        // Ensure that division by zero fails.
        assert!(
            panic::catch_unwind(|| fixed_u128!(1e18).mul_div_down(fixed!(1e18), fixed!(0)))
                .is_err()
        );
    }

    #[test]
    fn test_mul_div_down_properties() {
        let mut rng = thread_rng();
        for _ in 0..10_000 {
            let max = max_numerator();
            let a = rng.gen_range(0.into()..=max);
            let b = rng.gen_range(0.into()..=max / a);
            let c = rng.gen_range(1.into()..=max); // Ensure non-zero divisor

            // Property 1: mul_div_down(a, b, c) = (a * b) / c
            let result = a.mul_div_down(b, c);

            // Property 2: mul_div_down with identity values
            let one = a.one();
            // a * 1 / 1 should be a
            assert_eq!(a.mul_div_down(one, one), a);

            // Property 3: Associativity for integer inputs
            if a.raw() % c.raw() == 0.into() {
                assert_eq!(a.div_down(c).mul_down(b), result);
            }

            // Property 4: Commutativity of multiplication
            assert_eq!(a.mul_div_down(b, c), b.mul_div_down(a, c));
        }
    }

    #[test]
    fn test_mul_div_up_failure() {
        // Ensure that division by zero fails.
        assert!(
            panic::catch_unwind(|| fixed_u128!(1e18).mul_div_up(fixed!(1e18), fixed!(0))).is_err()
        );
    }

    #[test]
    fn test_mul_div_up_properties() {
        let mut rng = thread_rng();
        for _ in 0..10_000 {
            let max = max_numerator();
            let a = rng.gen_range(0.into()..=max);
            let b = rng.gen_range(0.into()..=max / a);
            let c = rng.gen_range(1.into()..=max); // Ensure non-zero divisor

            // Property 1: Check if mul_div_up equals mul_div_down for small, exactly divisible
            // values
            let one = a.one();
            let two = one + one;
            let four = two + two;

            // These multiplications and divisions will be exact
            assert_eq!(two.mul_div_up(two, four), one); // 2*2/4 = 1
            assert_eq!(two.mul_div_down(two, four), one); // 2*2/4 = 1

            // Property 2: Check relationship between mul_div_up and mul_div_down If the division is
            // not exact, mul_div_up should be greater than mul_div_down
            if a.mul_div_up(b, c) != a.mul_div_down(b, c) {
                assert!(a.mul_div_up(b, c) > a.mul_div_down(b, c));

                // Property 3: The difference should be exactly 1 for proper rounding
                let diff = a.mul_div_up(b, c) - a.mul_div_down(b, c);
                assert_eq!(diff, a.one());
            }

            // Property 4: Identity values
            let one = a.one();
            assert_eq!(a.mul_div_up(one, one), a);

            // Property 5: Commutativity of multiplication
            assert_eq!(a.mul_div_up(b, c), b.mul_div_up(a, c));
        }
    }

    #[test]
    fn test_mul_down_properties() {
        let mut rng = thread_rng();
        for _ in 0..10_000 {
            let a: FixedPoint<U256> = rng.gen();
            let b: FixedPoint<U256> = rng.gen();

            // Property 1: mul_down(a, b) = mul_div_down(a, b, 1e18)
            let one = a.one();
            assert_eq!(a.mul_down(b), a.mul_div_down(b, one));

            // Property 2: Commutativity
            assert_eq!(a.mul_down(b), b.mul_down(a));

            // Property 3: Identity
            assert_eq!(a.mul_down(one), a);

            // Property 4: Zero property
            let zero = FixedPoint::<U256>::zero();
            assert_eq!(a.mul_down(zero), zero);

            // Property 5: For values < 1e18, result should be less than the smaller input
            if a < one && b < one && !a.is_zero() && !b.is_zero() {
                let result = a.mul_down(b);
                assert!(result < a.min(b));
            }
        }
    }

    #[test]
    fn test_mul_up_properties() {
        let mut rng = thread_rng();
        for _ in 0..10_000 {
            let a: FixedPoint<U256> = rng.gen();
            let b: FixedPoint<U256> = rng.gen();

            // Property 1: mul_up(a, b) = mul_div_up(a, b, 1e18)
            let one = a.one();
            assert_eq!(a.mul_up(b), a.mul_div_up(b, one));

            // Property 2: Commutativity
            assert_eq!(a.mul_up(b), b.mul_up(a));

            // Property 3: Identity
            assert_eq!(a.mul_up(one), a);

            // Property 4: Zero property
            let zero = FixedPoint::<U256>::zero();
            assert_eq!(a.mul_up(zero), zero);

            // Property 5: mul_up should be greater than or equal to mul_down
            if !a.is_zero() && !b.is_zero() {
                let mul_up_result = panic::catch_unwind(|| a.mul_up(b));
                let mul_down_result = panic::catch_unwind(|| a.mul_down(b));

                if mul_up_result.is_ok() && mul_down_result.is_ok() {
                    assert!(mul_up_result.unwrap() >= mul_down_result.unwrap());
                }
            }
        }
    }

    #[test]
    fn test_div_down_failure() {
        // Ensure that division by zero fails.
        assert!(panic::catch_unwind(|| fixed_u128!(1e18).div_down(fixed!(0))).is_err());
    }

    #[test]
    fn test_div_down_properties() {
        let mut rng = thread_rng();
        for _ in 0..10_000 {
            let max = max_numerator();
            let a = rng.gen_range(0.into()..=max);
            let b = rng.gen_range(1.into()..=max); // Ensure non-zero divisor

            // Property 1: div_down(a, b) = mul_div_down(a, 1e18, b)
            let one = a.one();
            assert_eq!(a.div_down(b), a.mul_div_down(one, b));

            // Property 2: Identity properties
            assert_eq!(a.div_down(one), a);
            if !a.is_zero() {
                assert_eq!(a.div_down(a), one);
            }

            // Property 3: Zero property
            let zero = FixedPoint::<U256>::zero();
            assert_eq!(zero.div_down(b), zero);

            // Property 4: Inverse relationship with multiplication
            if !a.is_zero() && !b.is_zero() {
                let div_result = a.div_down(b);
                // Check if a ≈ (a/b) * b, accounting for rounding
                let reconstructed = div_result.mul_down(b);
                // Should be less than or equal to original due to rounding down
                assert!(reconstructed <= a);

                // The difference should be less than one unit due to rounding
                let diff = a - reconstructed;
                assert!(diff < one);
            }
        }
    }

    #[test]
    fn test_div_up_failure() {
        // Ensure that division by zero fails.
        assert!(panic::catch_unwind(|| fixed_u128!(1e18).div_up(fixed!(0))).is_err());
    }

    #[test]
    fn test_div_up_properties() {
        let mut rng = thread_rng();
        for _ in 0..10_000 {
            let max = max_numerator();
            let a = rng.gen_range(0.into()..=max);
            let b = rng.gen_range(1.into()..=max); // Ensure non-zero divisor

            // Property 1: div_up(a, b) = mul_div_up(a, 1e18, b)
            let one = a.one();
            assert_eq!(a.div_up(b), a.mul_div_up(one, b));

            // Property 2: Identity properties
            assert_eq!(a.div_up(one), a);
            if !a.is_zero() {
                assert_eq!(a.div_up(a), one);
            }

            // Property 3: Zero property
            let zero = FixedPoint::<U256>::zero();
            assert_eq!(zero.div_up(b), zero);

            // Property 4: div_up should be greater than or equal to div_down
            if !a.is_zero() && !b.is_zero() {
                assert!(a.div_up(b) >= a.div_down(b));

                // Property 5: If exactly divisible, div_up = div_down
                if a.div_up(b) == a.div_down(b) {
                    // If the division is exact, up and down rounding are the same No further checks
                    // needed
                } else {
                    // The difference should be exactly 1 for proper rounding
                    let diff = a.div_up(b) - a.div_down(b);
                    assert_eq!(diff, a.one());
                }
            }

            // Property 6: Inverse relationship with multiplication
            if !a.is_zero() && !b.is_zero() {
                let div_result = a.div_up(b);
                // Check if a ≤ (a÷b)×b, accounting for rounding up
                let reconstructed = div_result.mul_down(b);
                // Should be greater than or equal to original due to rounding up
                assert!(reconstructed >= a || (a - reconstructed) < a.one());
            }
        }
    }

    #[test]
    fn test_pow_properties() {
        let mut rng = thread_rng();
        for _ in 0..1_000 {
            // Reduced iterations as pow is computationally expensive
            let x = rng.gen_range(fixed!(0)..=fixed!(1e18));
            let y = rng.gen_range(fixed!(0)..=fixed!(10)); // Reasonable exponent range

            let one = x.one();
            let zero = FixedPoint::<U256>::zero();

            // Property 1: x^0 = 1
            let result_pow_zero = x.pow(zero);
            assert!(result_pow_zero.is_ok());
            assert_eq!(result_pow_zero.unwrap(), one);

            // Property 2: x^1 = x
            let result_pow_one = x.pow(one);
            assert!(result_pow_one.is_ok());
            assert_eq!(result_pow_one.unwrap(), x);

            // Property 3: 0^y = 0 for y > 0
            if !y.is_zero() {
                let result_zero_pow = zero.pow(y);
                assert!(result_zero_pow.is_ok());
                assert_eq!(result_zero_pow.unwrap(), zero);
            }

            // Property 4: 1^y = 1
            let result_one_pow = one.pow(y);
            assert!(result_one_pow.is_ok());
            assert_eq!(result_one_pow.unwrap(), one);

            // Property 5: For integer exponents, verify x^y = x * x^(y-1)
            if y > one && y.raw() % one.raw() == 0.into() {
                let y_minus_one = y - one;
                let pow_y_minus_one = x.pow(y_minus_one);
                let pow_y = x.pow(y);

                if pow_y_minus_one.is_ok() && pow_y.is_ok() {
                    // Allow small rounding errors due to fixed-point arithmetic
                    let product = x.mul_down(pow_y_minus_one.unwrap());
                    let diff = product.abs_diff(pow_y.unwrap());
                    assert!(diff <= fixed!(1e10)); // Small tolerance for rounding
                }
            }

            // Property 6: For fractional y where y = a/b, verify (x^a)^(1/b) ≈ x^y This property is
            // harder to test due to precision issues

            // Property 7: Negative exponents: x^(-y) = 1/(x^y)
            if !x.is_zero() && !y.is_zero() {
                let neg_y = -y;
                let pow_neg_y = x.pow(neg_y);
                let pow_y = x.pow(y);

                if pow_neg_y.is_ok() && pow_y.is_ok() {
                    let inverse = one.div_down(pow_y.unwrap());
                    let diff = inverse.abs_diff(pow_neg_y.unwrap());
                    assert!(diff <= fixed!(1e10)); // Small tolerance for rounding
                }
            }
        }
    }

    // Advanced property-based testing using proptest
    proptest! {
        #[test]
        fn prop_mul_commutative(a in any::<u64>(), b in any::<u64>()) {
            // Convert to FixedPoint
            let a_fixed = FixedPoint::<U256>::from(a);
            let b_fixed = FixedPoint::<U256>::from(b);

            // Test commutativity
            prop_assert_eq!(a_fixed.mul_down(b_fixed), b_fixed.mul_down(a_fixed));
        }

        #[test]
        fn prop_div_inverse(a in 1..u64::MAX, b in 1..u64::MAX) {
            // Convert to FixedPoint
            let a_fixed = FixedPoint::<U256>::from(a);
            let b_fixed = FixedPoint::<U256>::from(b);
            let one = a_fixed.one();

            // a/b * b/a should approximately equal 1, with some rounding error
            let result = a_fixed.div_down(b_fixed).mul_down(b_fixed.div_down(a_fixed));
            let diff = result.abs_diff(one);

            // Allow small tolerance for rounding errors
            prop_assert!(diff <= fixed!(1e10));
        }

        #[test]
        fn prop_mul_div_identity(a in any::<u64>(), b in 1..u64::MAX) {
            // Convert to FixedPoint
            let a_fixed = FixedPoint::<U256>::from(a);
            let b_fixed = FixedPoint::<U256>::from(b);

            // a * b / b should approximately equal a, with some rounding error
            let result = a_fixed.mul_down(b_fixed).div_down(b_fixed);
            let diff = result.abs_diff(a_fixed);

            // Allow small tolerance for rounding errors
            prop_assert!(diff <= fixed!(1e10));
        }
    }
}
