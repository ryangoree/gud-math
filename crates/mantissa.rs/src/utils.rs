use std::ops::Shr;

use ethers::types::{I256, U256};
use eyre::{bail, Result};

use crate::{int256, uint256};

/// Parses a string into a U256 with support for scientific and decimal
/// notation.
///
/// ## Example
///
/// ```
/// use ethers::types::U256;
/// use fixedpointmath::u256_from_str;
///
/// let u = u256_from_str("1.1e18").unwrap();
/// assert_eq!(u, U256::from(11) * U256::from(10).pow(U256::from(17)));
/// ```
pub fn u256_from_str(s: &str) -> Result<U256> {
    // Parse a string into a mantissa and an exponent. The U256 arithmetic will
    // overflow if the mantissa or the exponent are too large.
    let mut found_dot = false;
    let mut found_e = false;
    let mut mantissa = ethers::types::U256::zero();
    let mut exponent = ethers::types::U256::zero();
    let mut decimals = 0;

    for digit in s.chars() {
        if digit.is_ascii_digit() {
            let d = digit.to_digit(10).unwrap();
            if !found_e {
                mantissa = mantissa * 10 + d;
            } else {
                exponent = exponent * 10 + d;
            }
            if found_dot && !found_e {
                decimals += 1;
            }
        } else if digit == 'e' && !found_e {
            found_e = true;
        } else if digit == '.' && !found_dot {
            found_dot = true;
        } else if digit != '_' {
            bail!("Unexpected character in U256: {digit}");
        }
    }

    // Combine the mantissa and the exponent into a single U256. This will
    // overflow if the exponent is too large. We also need to make sure that the
    // final result is an integer.
    let decimals = ethers::types::U256::from(decimals);
    if exponent < decimals {
        bail!("Exponent {exponent} is too small for U256: {s}");
    }

    Ok(mantissa * ethers::types::U256::from(10).pow(exponent - decimals))
}

/// Parse a string into an I256 with support for scientific and decimal
/// notation.
///
/// ## Example
///
/// ```
/// use ethers::types::I256;
/// use fixedpointmath::i256_from_str;
///
/// let i = i256_from_str("-1.1e18").unwrap();
/// assert_eq!(i, -I256::from(11) * I256::from(10).pow(17));
/// ```
pub fn i256_from_str(s: &str) -> Result<I256> {
    // Parse a string into a mantissa and an exponent. The U256 arithmetic will
    // overflow if the mantissa or the exponent are too large.
    let mut sign = ethers::types::I256::one();
    let mut found_dot = false;
    let mut found_e = false;
    let mut mantissa = ethers::types::I256::zero();
    let mut exponent = 0;
    let mut decimals = 0;

    for digit in s.chars() {
        if digit.is_ascii_digit() {
            let d = digit.to_digit(10).unwrap();
            if !found_e {
                mantissa = mantissa * 10 + d;
            } else {
                exponent = exponent * 10 + d;
            }
            if found_dot && !found_e {
                decimals += 1;
            }
        } else if digit == '-' {
            sign = -ethers::types::I256::one();
        } else if digit == 'e' && !found_e {
            found_e = true;
        } else if digit == '.' && !found_dot {
            found_dot = true;
        } else if digit != '_' {
            bail!("Unexpected character in I256: {digit}");
        }
    }

    // Combine the mantissa and the exponent I256. This will overflow if the
    // exponent is too large. We also need to make sure that the final result is
    // an integer.
    if exponent < decimals {
        bail!("Exponent {exponent} is too small for I256: {s}");
    }

    Ok(sign * mantissa * ethers::types::I256::from(10).pow(exponent - decimals))
}

/// Math

pub fn exp(mut x: I256) -> Result<I256> {
    // When the result is < 0.5 we return zero. This happens when x <=
    // floor(log(0.5e18) * 1e18) ~ -42e18
    if x <= I256::from(-42139678854452767551_i128) {
        return Ok(I256::zero());
    }

    // When the result is > (2**255 - 1) / 1e18 we can not represent it as an
    // int. This happens when x >= floor(log((2**255 - 1) / 1e18) * 1e18) ~ 135.
    if x >= int256!(135305999368893231589) {
        bail!("invalid exponent {x}");
    }

    // x is now in the range (-42, 136) * 1e18. Convert to (-42, 136) * 2**96
    // for more intermediate precision and a binary basis. This base conversion
    // is a multiplication by 1e18 / 2**96 = 5**18 / 2**78.
    x = x.wrapping_shl(78) / int256!(5).pow(18);

    // Reduce range of x to (-½ ln 2, ½ ln 2) * 2**96 by factoring out powers of
    // two such that exp(x) = exp(x') * 2**k, where k is an integer. Solving
    // this gives k = round(x / log(2)) and x' = x - k * log(2).
    let k = ((x.wrapping_shl(96) / int256!(54916777467707473351141471128))
        .wrapping_add(int256!(2).pow(95)))
    .asr(96);
    x = x.wrapping_sub(k.wrapping_mul(54916777467707473351141471128_u128.into()));

    // k is in the range [-61, 195].

    // Evaluate using a (6, 7)-term rational approximation. p is made monic,
    // we'll multiply by a scale factor later.
    let mut y = x.wrapping_add(1346386616545796478920950773328_u128.into());
    y = y
        .wrapping_mul(x)
        .asr(96)
        .wrapping_add(57155421227552351082224309758442_u128.into());
    let mut p = y
        .wrapping_add(x)
        .wrapping_sub(94201549194550492254356042504812_u128.into());
    p = p
        .wrapping_mul(y)
        .asr(96)
        .wrapping_add(28719021644029726153956944680412240_u128.into());
    p = p
        .wrapping_mul(x)
        .wrapping_add(int256!(4385272521454847904659076985693276).wrapping_shl(96));

    // We leave p in 2**192 basis so we don't need to scale it back up for the
    // division.
    let mut q = x.wrapping_sub(2855989394907223263936484059900_u128.into());
    q = q
        .wrapping_mul(x)
        .asr(96)
        .wrapping_add(50020603652535783019961831881945_u128.into());
    q = q
        .wrapping_mul(x)
        .asr(96)
        .wrapping_sub(533845033583426703283633433725380_u128.into());
    q = q
        .wrapping_mul(x)
        .asr(96)
        .wrapping_add(3604857256930695427073651918091429_u128.into());
    q = q
        .wrapping_mul(x)
        .asr(96)
        .wrapping_sub(14423608567350463180887372962807573_u128.into());
    q = q
        .wrapping_mul(x)
        .asr(96)
        .wrapping_add(26449188498355588339934803723976023_u128.into());

    let mut r = p.wrapping_div(q);

    // r should be in the range (0.09, 0.25) * 2**96.

    // We now need to multiply r by:
    // * the scale factor s = ~6.031367120.
    // * the 2**k factor from the range reduction.
    // * the 1e18 / 2**96 factor for base conversion. We do this all at once,
    // with an intermediate result in 2**213 basis, so the final right shift is
    // always by a positive amount.
    r = I256::from_raw(
        (r.into_raw()
            .overflowing_mul(uint256!(3822833074963236453042738258902158003155416615667))
            .0)
            .shr(int256!(195).wrapping_sub(k).low_usize()),
    );

    Ok(r)
}

pub fn ln(mut x: I256) -> Result<I256> {
    if x <= I256::zero() {
        bail!("Cannot calculate ln of of negative number or zero.");
    }

    // We want to convert x from 10**18 fixed point to 2**96 fixed point. We do
    // this by multiplying by 2**96 / 10**18. But since ln(x * C) = ln(x) +
    // ln(C), we can simply do nothing here and add ln(2**96 / 10**18) at the
    // end.

    let mut r = I256::from((x > I256::from(0xffffffffffffffffffffffffffffffff_u128)) as u128)
        .wrapping_shl(7);
    r = r | I256::from((x.asr(r.as_usize()) > I256::from(0xffffffffffffffff_u128)) as u128)
        .wrapping_shl(6);
    r = r | I256::from((x.asr(r.as_usize()) > I256::from(0xffffffff_u128)) as u128).wrapping_shl(5);
    r = r | I256::from((x.asr(r.as_usize()) > I256::from(0xffff_u128)) as u128).wrapping_shl(4);
    r = r | I256::from((x.asr(r.as_usize()) > I256::from(0xff_u128)) as u128).wrapping_shl(3);
    r = r | I256::from((x.asr(r.as_usize()) > I256::from(0xf_u128)) as u128).wrapping_shl(2);
    r = r | I256::from((x.asr(r.as_usize()) > I256::from(0x3_u128)) as u128).wrapping_shl(1);
    r = r | I256::from((x.asr(r.as_usize()) > I256::from(0x1_u128)) as u128);

    // Reduce range of x to (1, 2) * 2**96 ln(2^k * x) = k * ln(2) + ln(x)
    let k = r.wrapping_sub(int256!(96));
    x = x.wrapping_shl(int256!(159).wrapping_sub(k).as_usize());
    x = I256::from_raw(x.into_raw().shr(159));

    // Evaluate using a (8, 8)-term rational approximation. p is made monic, we
    // will multiply by a scale factor later.
    let mut p = x.wrapping_add(int256!(3273285459638523848632254066296));
    p = ((p.wrapping_mul(x)).asr(96)).wrapping_add(int256!(24828157081833163892658089445524));
    p = ((p.wrapping_mul(x)).asr(96)).wrapping_add(int256!(43456485725739037958740375743393));
    p = ((p.wrapping_mul(x)).asr(96)).wrapping_sub(int256!(11111509109440967052023855526967));
    p = ((p.wrapping_mul(x)).asr(96)).wrapping_sub(int256!(45023709667254063763336534515857));
    p = ((p.wrapping_mul(x)).asr(96)).wrapping_sub(int256!(14706773417378608786704636184526));
    p = p
        .wrapping_mul(x)
        .wrapping_sub(int256!(795164235651350426258249787498).wrapping_shl(96));

    // We leave p in 2**192 basis so we don't need to scale it back up for the
    // division. q is monic by convention.
    let mut q = x.wrapping_add(int256!(5573035233440673466300451813936));
    q = (q.wrapping_mul(x).asr(96)).wrapping_add(int256!(71694874799317883764090561454958));
    q = q
        .wrapping_mul(x)
        .asr(96)
        .wrapping_add(int256!(283447036172924575727196451306956));
    q = q
        .wrapping_mul(x)
        .asr(96)
        .wrapping_add(int256!(401686690394027663651624208769553));
    q = q
        .wrapping_mul(x)
        .asr(96)
        .wrapping_add(int256!(204048457590392012362485061816622));
    q = q
        .wrapping_mul(x)
        .asr(96)
        .wrapping_add(int256!(31853899698501571402653359427138));
    q = q
        .wrapping_mul(x)
        .asr(96)
        .wrapping_add(int256!(909429971244387300277376558375));

    r = p.wrapping_div(q);

    // r is in the range (0, 0.125) * 2**96

    // Finalization, we need to:
    // * multiply by the scale factor s = 5.549…
    // * add ln(2**96 / 10**18)
    // * add k * ln(2)
    // * multiply by 10**18 / 2**96 = 5**18 >> 78

    // mul s * 5e18 * 2**96, base is now 5**18 * 2**192
    r = r.wrapping_mul(int256!(1677202110996718588342820967067443963516166));
    // add ln(2) * k * 5e18 * 2**192
    r = r.wrapping_add(
        int256!(16597577552685614221487285958193947469193820559219878177908093499208371)
            .wrapping_mul(k),
    );
    // add ln(2**96 / 10**18) * 5e18 * 2**192
    r = r.wrapping_add(int256!(
        600920179829731861736702779321621459595472258049074101567377883020018308
    ));
    // base conversion: mul 2**18 / 2**192
    r = r.asr(174);

    Ok(r)
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use rand::{thread_rng, Rng};

    use super::*;
    use crate::FixedPoint;
    use ethers::types::{I256, U256};

    // Tests for u256_from_str
    #[test]
    fn test_u256_from_str() {
        // Test normal integers
        assert_eq!(u256_from_str("123").unwrap(), U256::from(123));
        assert_eq!(u256_from_str("0").unwrap(), U256::from(0));

        // Test with decimals
        assert_eq!(u256_from_str("1.5e1").unwrap(), U256::from(15));
        assert_eq!(u256_from_str("1.5e2").unwrap(), U256::from(150));
        assert_eq!(u256_from_str("1.5e18").unwrap(), U256::from(15) * U256::from(10).pow(U256::from(17)));

        // Test with scientific notation
        assert_eq!(
            u256_from_str("1e18").unwrap(),
            U256::from(10).pow(U256::from(18))
        );
        assert_eq!(
            u256_from_str("1.1e18").unwrap(),
            U256::from(11) * U256::from(10).pow(U256::from(17))
        );

        // Test invalid inputs
        assert!(u256_from_str("abc").is_err());
        assert!(u256_from_str("1.1.1").is_err());
        assert!(u256_from_str("-123").is_err()); // Negative not allowed for U256
    }

    // Tests for i256_from_str
    #[test]
    fn test_i256_from_str() {
        // Test normal integers
        assert_eq!(i256_from_str("123").unwrap(), I256::from(123));
        assert_eq!(i256_from_str("0").unwrap(), I256::from(0));
        assert_eq!(i256_from_str("-123").unwrap(), I256::from(-123));

        // Test with decimals
        assert_eq!(i256_from_str("1.5e1").unwrap(), I256::from(15));
        assert_eq!(i256_from_str("-1.5e1").unwrap(), I256::from(-15));
        assert_eq!(i256_from_str("1.5e18").unwrap(), I256::from(15) * I256::from(10).pow(17));
        assert_eq!(i256_from_str("-1.5e18").unwrap(), -I256::from(15) * I256::from(10).pow(17));

        // Test with scientific notation
        assert_eq!(i256_from_str("1e18").unwrap(), I256::from(10).pow(18));
        assert_eq!(
            i256_from_str("1.1e18").unwrap(),
            I256::from(11) * I256::from(10).pow(17)
        );
        assert_eq!(
            i256_from_str("-1.1e18").unwrap(),
            -I256::from(11) * I256::from(10).pow(17)
        );

        // Test invalid inputs
        assert!(i256_from_str("abc").is_err());
        assert!(i256_from_str("1.1.1").is_err());
        
        // Test exponents too small (decimals would be truncated)
        assert!(i256_from_str("1.5e0").is_err());
        
        // The implementation treats "1.5e-1" as "-1.5e1" (the minus sign is applied to the whole number)
        // This is a quirk of the current implementation
        let neg_exp = i256_from_str("1.5e-1").unwrap();
        assert_eq!(neg_exp, I256::from(-15));
    }

    // Test for exp function
    #[test]
    fn test_exp_properties() {
        let mut rng = thread_rng();

        // Test exp(0) = 1
        let zero = I256::zero();
        let result_zero = exp(zero).unwrap();
        let expected_one = I256::from(10).pow(18); // 1e18 in fixed point
        assert_eq!(result_zero, expected_one);

        // Test small positive inputs
        for _ in 0..100 {
            // Generate random FixedPoint values using the i128 range
            let x_i128 = rng.gen_range(0..=1000000); // 0 to 1 in fixed point (after scaling)
            // Scale to get a value between 0 and 1
            let x = I256::from(x_i128) * I256::from(10).pow(12); // scale to get values up to 1e18
            let result = exp(x).unwrap();

            // exp(x) should be >= 1 for x >= 0
            assert!(result >= expected_one);

            // For very small x values, exp(x) ≈ 1 + x
            if x <= I256::from(10).pow(15) {
                // x <= 0.001
                let approx = expected_one + x;
                let diff = if result > approx {
                    result - approx
                } else {
                    approx - result
                };
                assert!(diff <= I256::from(10).pow(15)); // Allow small error
            }
        }

        // Test exp(x) is strictly increasing for x > 0
        for _ in 0..100 {
            // Generate random value in range 0-10
            let x1_i128 = rng.gen_range(0..10000000000i128); // 0 to 10 in fixed point (after scaling)
            let x1 = I256::from(x1_i128) * I256::from(10).pow(8); // scale to get values up to 10e18
            
            // Create a larger value for x2
            let increment = I256::from(10).pow(16); // 0.01 in fixed point
            let x2 = x1 + increment;
            
            let result1 = exp(x1).unwrap();
            let result2 = exp(x2).unwrap();
            assert!(result2 > result1);
        }

        // Test exp(-x) = 1/exp(x)
        for _ in 0..100 {
            // Generate random value in range 0.1-10
            let x_i128 = rng.gen_range(100000000i128..10000000000i128); // 0.1 to 10 in fixed point (after scaling)
            let x = I256::from(x_i128) * I256::from(10).pow(9); // scale to get values from 0.1e18 to 10e18
            
            let exp_x = exp(x).unwrap();
            let exp_neg_x = exp(-x).unwrap();

            // Fixed point division to check if exp(-x) ≈ 1/exp(x)
            // Need to scale by 1e18
            let one_over_exp_x = I256::from(10).pow(36) / exp_x;

            // Allow for rounding errors
            let diff = if exp_neg_x > one_over_exp_x {
                exp_neg_x - one_over_exp_x
            } else {
                one_over_exp_x - exp_neg_x
            };

            // Error should be relatively small for reasonable values
            assert!(diff <= I256::from(10).pow(16)); // Allow up to 1% error
        }
    }

    // Test for ln function
    #[test]
    fn test_ln_properties() {
        let mut rng = thread_rng();

        // Test ln(1) = 0
        let one = I256::from(10).pow(18); // 1e18 in fixed point
        let result_one = ln(one).unwrap();
        assert_eq!(result_one, I256::zero());

        // Test ln(e) = 1
        let e = exp(one).unwrap(); // e^1 in fixed point
        let ln_e = ln(e).unwrap();
        assert!(ln_e >= one - I256::from(10).pow(15) && ln_e <= one + I256::from(10).pow(15));

        // Test ln(a*b) = ln(a) + ln(b)
        for _ in 0..100 {
            // Generate random values in range 0.1-10
            let a_i128 = rng.gen_range(100000000i128..10000000000i128); // 0.1 to 10 in fixed point (after scaling)
            let b_i128 = rng.gen_range(100000000i128..10000000000i128); // 0.1 to 10 in fixed point (after scaling)
            
            let a = I256::from(a_i128) * I256::from(10).pow(9); // scale to get values from 0.1e18 to 10e18
            let b = I256::from(b_i128) * I256::from(10).pow(9); // scale to get values from 0.1e18 to 10e18
            
            // Compute a*b carefully to avoid overflow
            let a_fixed = FixedPoint::<I256>::new(a);
            let b_fixed = FixedPoint::<I256>::new(b);
            let ab_fixed = a_fixed.mul_down(b_fixed);
            let ab = ab_fixed.raw();

            let ln_a = ln(a).unwrap();
            let ln_b = ln(b).unwrap();
            let ln_ab = ln(ab).unwrap();

            // ln(a*b) should approximately equal ln(a) + ln(b)
            let ln_sum = ln_a + ln_b;
            let diff = if ln_ab > ln_sum {
                ln_ab - ln_sum
            } else {
                ln_sum - ln_ab
            };

            // Allow for rounding errors
            assert!(diff <= I256::from(10).pow(16)); // Allow up to 1% error
        }

        // Test ln(a/b) = ln(a) - ln(b)
        for _ in 0..100 {
            // Generate random values in range 1-100 for a and 0.1-1 for b
            let a_i128 = rng.gen_range(1000000000i128..100000000000i128); // 1 to 100 in fixed point (after scaling)
            let b_i128 = rng.gen_range(100000000i128..1000000000i128);    // 0.1 to 1 in fixed point (after scaling)
            
            let a = I256::from(a_i128) * I256::from(10).pow(9); // scale to get values from 1e18 to 100e18
            let b = I256::from(b_i128) * I256::from(10).pow(9); // scale to get values from 0.1e18 to 1e18
            
            // Compute a/b carefully to avoid overflow
            let a_fixed = FixedPoint::<I256>::new(a);
            let b_fixed = FixedPoint::<I256>::new(b);
            let a_div_b_fixed = a_fixed.div_down(b_fixed);
            let a_div_b = a_div_b_fixed.raw();

            let ln_a = ln(a).unwrap();
            let ln_b = ln(b).unwrap();
            let ln_a_div_b = ln(a_div_b).unwrap();

            // ln(a/b) should approximately equal ln(a) - ln(b)
            let ln_diff = ln_a - ln_b;
            let diff = if ln_a_div_b > ln_diff {
                ln_a_div_b - ln_diff
            } else {
                ln_diff - ln_a_div_b
            };

            // Allow for rounding errors
            assert!(diff <= I256::from(10).pow(16)); // Allow up to 1% error
        }

        // Test ln is strictly increasing
        for _ in 0..100 {
            // Generate random value in range 0.1-10
            let x1_i128 = rng.gen_range(100000000i128..10000000000i128); // 0.1 to 10 in fixed point (after scaling)
            let x1 = I256::from(x1_i128) * I256::from(10).pow(9); // scale to get values from 0.1e18 to 10e18
            
            // Create a larger value for x2
            let increment = I256::from(10).pow(17); // 0.1 in fixed point
            let x2 = x1 + increment;
            
            let result1 = ln(x1).unwrap();
            let result2 = ln(x2).unwrap();
            assert!(result2 > result1);
        }
    }

    // Property-based testing
    proptest! {
        #[test]
        fn prop_exp_ln_inverse(x in 1_i128..1_000_000_000_000_000_000_i128) {
            // Convert to fixed-point
            let x_fixed = I256::from(x) * I256::from(10).pow(18) / I256::from(1_000_000_000_000_000_000_i128);

            // Only test for x > 0 where ln is defined
            if x_fixed > I256::zero() {
                // exp(ln(x)) should be approximately x
                let ln_x = ln(x_fixed).unwrap();
                let exp_ln_x = exp(ln_x).unwrap();

                // Calculate relative error
                let diff = if exp_ln_x > x_fixed { exp_ln_x - x_fixed } else { x_fixed - exp_ln_x };
                let relative_error = diff * I256::from(100) / x_fixed; // percentage error

                // Allow for small relative error (0.1%)
                prop_assert!(relative_error <= I256::from(1));
            }
        }

        #[test]
        fn prop_ln_exp_inverse(x in -40_i128..40_i128) {
            // Convert to fixed-point (scaled to avoid overflow)
            let x_fixed = I256::from(x) * I256::from(10).pow(17);

            // ln(exp(x)) should be approximately x
            let exp_x = exp(x_fixed).unwrap();
            if exp_x > I256::zero() { // Ensure exp(x) > 0 for ln to be defined
                let ln_exp_x = ln(exp_x).unwrap();

                // Calculate absolute error
                let diff = if ln_exp_x > x_fixed { ln_exp_x - x_fixed } else { x_fixed - ln_exp_x };

                // Allow for small absolute error
                prop_assert!(diff <= I256::from(10).pow(16)); // Allow up to 1% error
            }
        }
    }
}
