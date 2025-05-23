pub mod formatting;
pub mod utils;

use core::fmt;
use std::str::FromStr;

use ethers::types::I256;
use js_sys::{parse_float, BigInt, JsString};
use fixed::{i256_from_str, ln, prelude::*};
use rand::{thread_rng, Rng};
use ts_macro::ts;
use utils_core::{
    conversions::ToI256,
    error,
    error::{Error, ToResult},
    type_error,
};
use wasm_bindgen::prelude::*;

// Initialization function
#[wasm_bindgen(start)]
pub fn initialize() {
    // When the `console_error_panic_hook` feature is enabled, we can call the
    // `set_panic_hook` function at least once during initialization, and then
    // we will get better error messages if our code ever panics.
    //
    // For more details see
    // https://github.com/rustwasm/console_error_panic_hook#readme
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

const INNER_DECIMALS: u8 = 18;

/// A number with a fixed number of decimal places.
///
/// @example
/// ```js
/// const formattedSpotPrice = new FixedPoint(initialSharePrice)
///   .mulDivDown(effectiveShareReserves, bondReserves)
///   .pow(timeStretch)
///   .format();
/// ```
#[wasm_bindgen(js_name = FixedPoint)]
pub struct WasmFixedPoint {
    inner: FixedPoint<I256>,
    /// The number of decimal places in the fixed-point number.
    #[wasm_bindgen(readonly)]
    pub decimals: u8,
}

impl fmt::Display for WasmFixedPoint {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.inner)
    }
}

// Methods //

#[wasm_bindgen(js_class = FixedPoint)]
impl WasmFixedPoint {
    // Constructors //

    #[wasm_bindgen(constructor, skip_jsdoc)]
    pub fn new(value: Numberish, decimals: Option<u8>) -> Result<WasmFixedPoint, Error> {
        let decimals = decimals.unwrap_or(INNER_DECIMALS);
        let is_fixed_point = value.is_fixed_point();
        let mut fixed_value: FixedPoint<I256> = value.try_into()?;

        if is_fixed_point == Some(true) {
            fixed_value /= WasmFixedPoint::scale_factor(decimals);
        }

        Ok(WasmFixedPoint {
            inner: fixed_value * WasmFixedPoint::scale_factor(decimals),
            decimals,
        })
    }

    /// Create a new fixed-point number from a scaled value or another
    /// fixed-point number. If the value is already a fixed-point number, the
    /// number of decimal places will be adjusted to match the new value.
    ///
    /// @param value - A scaled value between `-2^255` and `2^255 - 1` (signed
    /// 256-bit integer, i.e., `int256`).
    ///
    /// @param decimals - The number of decimal places to use. Max is `18`.
    /// Defaults to `18`.
    ///
    /// @example
    /// ```js
    /// const fromBigint = FixedPoint.from(1500000000000000000n);
    /// const fromNumber = FixedPoint.from(1.5e18);
    /// const fromString = FixedPoint.from('1.5e18');
    /// const withDecimals = FixedPoint.from(1.5e6, 6);
    ///
    /// console.log(fromBigint.toString());
    /// // => 1.500000000000000000
    ///
    /// console.log(fromNumber.toString());
    /// // => 1.500000000000000000
    ///
    /// console.log(fromString.toString());
    /// // => 1.500000000000000000
    ///
    /// console.log(withDecimals.toString());
    /// // => 1.500000
    /// ```
    #[wasm_bindgen(skip_jsdoc, js_name = from)]
    pub fn _from(value: Numberish, decimals: Option<u8>) -> Result<WasmFixedPoint, Error> {
        WasmFixedPoint::new(value, decimals)
    }

    /// Create a fixed-point number by parsing a decimal value and scaling it by
    /// a given number of decimal places.
    ///
    /// @param value - A value to parse and scale.
    ///
    /// @param decimals - The number of decimal places to use. Max is `18`.
    /// Defaults to `18`.
    ///
    /// @example
    //
    /// ```js
    /// const fromNumber = FixedPoint.parse(1.5);
    /// const fromString = FixedPoint.parse('1.5');
    /// const withDecimals = FixedPoint.parse('1.5', 6);
    ///
    /// console.log(fromNumber.toString());
    /// // => 1.500000000000000000
    ///
    /// console.log(fromString.toString());
    /// // => 1.500000000000000000
    ///
    /// console.log(withDecimals.toString());
    /// // => 1.500000
    /// ```
    #[wasm_bindgen(skip_jsdoc)]
    pub fn parse(value: Numberish, decimals: Option<u8>) -> Result<WasmFixedPoint, Error> {
        // If the value is already a fixed-point number, it's already scaled.
        if value.is_fixed_point() == Some(true) {
            return WasmFixedPoint::new(value, decimals);
        }

        let decimals = decimals.unwrap_or(INNER_DECIMALS);
        let mut s = value.to_string().as_string().unwrap_or_default();

        if s.contains("e") {
            let mut parts = s.split("e");
            let mantissa_str = parts.next().unwrap_or_default();
            let exponent_str = parts.next().unwrap_or_default();
            let exponent = i8::from_str_radix(exponent_str, 10).to_result()?;
            let mut mantissa_parts = mantissa_str.split(".");
            let int_str = mantissa_parts.next().unwrap_or_default();
            let fraction_str = mantissa_parts.next().unwrap_or_default();
            let mut inner = I256::from_dec_str(&format!("{int_str}{fraction_str}"))
                .to_result()?
                .fixed();
            let fraction_digits = fraction_str.len() as i8;

            if fraction_digits > exponent {
                inner /= (10u128.pow((fraction_digits - exponent) as u32)).into();
                return Ok(WasmFixedPoint { inner, decimals });
            }

            let unscaled = inner.raw() * I256::from(10u32.pow((exponent - fraction_digits) as u32));
            s = unscaled.to_string();
        } else if !s.contains(".") {
            let unscaled_value: FixedPoint<I256> = value.try_into()?;
            s = unscaled_value.raw().to_string();
        }

        let scaled_str = format!("{s}e{decimals}");
        WasmFixedPoint::new(JsValue::from(scaled_str).into(), Some(decimals))
    }

    /// Create a fixed-point number representing one unit.
    ///
    /// @param decimals - The number of decimal places to use. Max is `18`.
    /// Defaults to `18`.
    #[wasm_bindgen(skip_jsdoc)]
    pub fn one(decimals: Option<u8>) -> Result<WasmFixedPoint, Error> {
        let decimals = decimals.unwrap_or(INNER_DECIMALS);
        Ok(WasmFixedPoint {
            inner: FixedPoint::from(10u128.pow(decimals as u32)),
            decimals,
        })
    }

    /// Create a random fixed-point number with an optional min and max.
    ///
    /// **Note**:
    ///
    /// @example
    ///
    /// ```ts
    /// const random = FixedPoint.random();
    /// console.log(random.toString());
    /// // => 0.472987274007185487
    /// ```
    #[wasm_bindgen(skip_jsdoc)]
    pub fn random(params: Option<IGenerateRandomParams>) -> Result<WasmFixedPoint, Error> {
        let _params = match params {
            Some(params) => params.parse(),
            None => GenerateRandomParams {
                min: None,
                max: None,
                decimals: None,
            },
        };
        let fixed_min: FixedPoint<I256> = match _params.min {
            Some(min) => min.try_into()?,
            None => FixedPoint::zero(),
        };
        let fixed_max: FixedPoint<I256> = match _params.max {
            Some(max) => max.try_into()?,
            None => fixed_min + fixed_min.one(),
        };
        Ok(WasmFixedPoint {
            inner: thread_rng().gen_range(fixed_min..fixed_max),
            decimals: _params.decimals.unwrap_or(INNER_DECIMALS),
        })
    }

    // Getters //

    /// Get the scaled bigint representation of this fixed-point number.
    #[wasm_bindgen(skip_jsdoc, getter)]
    pub fn bigint(&self) -> Result<BigInt, Error> {
        let adjusted = self.inner / WasmFixedPoint::scale_factor(self.decimals);
        let string = format!("{:?}", adjusted.raw());
        BigInt::from_str(&string).map_err(|_| type_error!("Invalid BigInt: {}", string))
    }

    #[wasm_bindgen(skip_jsdoc, skip_typescript)]
    pub fn is_fixed_point(&self) -> bool {
        true
    }

    // Math //

    /// Get the absolute value of this fixed-point number.
    #[wasm_bindgen(skip_jsdoc)]
    pub fn abs(&self) -> Result<WasmFixedPoint, Error> {
        Ok(WasmFixedPoint {
            inner: self.inner.abs(),
            decimals: self.decimals,
        })
    }

    /// Get the absolute difference between this number and another.
    #[wasm_bindgen(skip_jsdoc, js_name = absDiff)]
    pub fn abs_diff(
        &self,
        other: Numberish,
        decimals: Option<u8>,
    ) -> Result<WasmFixedPoint, Error> {
        Ok(WasmFixedPoint {
            inner: self
                .inner
                .abs_diff(WasmFixedPoint::new(other, decimals)?.inner)
                .change_type()
                .to_result()?,
            decimals: self.decimals,
        })
    }

    /// Add a fixed-point number to this one.
    #[wasm_bindgen(skip_jsdoc)]
    pub fn add(&self, other: Numberish, decimals: Option<u8>) -> Result<WasmFixedPoint, Error> {
        Ok(WasmFixedPoint {
            inner: self.inner + WasmFixedPoint::new(other, decimals)?.inner,
            decimals: self.decimals,
        })
    }

    /// Subtract a fixed-point number from this one.
    #[wasm_bindgen(skip_jsdoc)]
    pub fn sub(&self, other: Numberish, decimals: Option<u8>) -> Result<WasmFixedPoint, Error> {
        Ok(WasmFixedPoint {
            inner: self.inner - WasmFixedPoint::new(other, decimals)?.inner,
            decimals: self.decimals,
        })
    }

    /// Multiply this fixed-point number by another, then divide by a divisor,
    /// rounding down.
    #[wasm_bindgen(skip_jsdoc, js_name = mulDivDown)]
    pub fn mul_div_down(
        &self,
        other: Numberish,
        divisor: Numberish,
        decimals: Option<u8>,
    ) -> Result<WasmFixedPoint, Error> {
        Ok(WasmFixedPoint {
            inner: self.inner.mul_div_down(
                WasmFixedPoint::new(other, decimals)?.inner,
                WasmFixedPoint::new(divisor, decimals)?.inner,
            ),
            decimals: self.decimals,
        })
    }

    /// Multiply this fixed-point number by another, then divide by a divisor,
    /// rounding up.
    #[wasm_bindgen(skip_jsdoc, js_name = mulDivUp)]
    pub fn mul_div_up(
        &self,
        other: Numberish,
        divisor: Numberish,
        decimals: Option<u8>,
    ) -> Result<WasmFixedPoint, Error> {
        Ok(WasmFixedPoint {
            inner: self.inner.mul_div_up(
                WasmFixedPoint::new(other, decimals)?.inner,
                WasmFixedPoint::new(divisor, decimals)?.inner,
            ),
            decimals: self.decimals,
        })
    }

    /// Multiply this fixed-point number by another, truncating the result.
    #[wasm_bindgen(skip_jsdoc, js_name = mulDown)]
    pub fn mul_down(
        &self,
        other: Numberish,
        decimals: Option<u8>,
    ) -> Result<WasmFixedPoint, Error> {
        Ok(WasmFixedPoint {
            inner: self
                .inner
                .mul_down(WasmFixedPoint::new(other, decimals)?.inner),
            decimals: self.decimals,
        })
    }

    /// Multiply this fixed-point number by another, rounding up.
    #[wasm_bindgen(skip_jsdoc, js_name = mulUp)]
    pub fn mul_up(&self, other: Numberish, decimals: Option<u8>) -> Result<WasmFixedPoint, Error> {
        Ok(WasmFixedPoint {
            inner: self
                .inner
                .mul_up(WasmFixedPoint::new(other, decimals)?.inner),
            decimals: self.decimals,
        })
    }

    /// Multiply this fixed-point number by another. Rounding to the nearest integer.
    #[wasm_bindgen(skip_jsdoc)]
    pub fn mul(&self, other: Numberish, decimals: Option<u8>) -> Result<WasmFixedPoint, Error> {
        Ok(WasmFixedPoint {
            inner: self.inner * WasmFixedPoint::new(other, decimals)?.inner,
            decimals: self.decimals,
        })
    }

    /// Divide this fixed-point number by another, truncating the result.
    #[wasm_bindgen(skip_jsdoc, js_name = divDown)]
    pub fn div_down(
        &self,
        other: Numberish,
        decimals: Option<u8>,
    ) -> Result<WasmFixedPoint, Error> {
        Ok(WasmFixedPoint {
            inner: self
                .inner
                .div_down(WasmFixedPoint::new(other, decimals)?.inner),
            decimals: self.decimals,
        })
    }

    /// Divide this fixed-point number by another, rounding up.
    #[wasm_bindgen(skip_jsdoc, js_name = divUp)]
    pub fn div_up(&self, other: Numberish, decimals: Option<u8>) -> Result<WasmFixedPoint, Error> {
        Ok(WasmFixedPoint {
            inner: self
                .inner
                .div_up(WasmFixedPoint::new(other, decimals)?.inner),
            decimals: self.decimals,
        })
    }

    /// Divide this fixed-point number by another.
    #[wasm_bindgen(skip_jsdoc)]
    pub fn div(&self, other: Numberish, decimals: Option<u8>) -> Result<WasmFixedPoint, Error> {
        Ok(WasmFixedPoint {
            inner: self.inner / WasmFixedPoint::new(other, decimals)?.inner,
            decimals: self.decimals,
        })
    }

    /// Raise this fixed-point number to the power of another.
    #[wasm_bindgen(skip_jsdoc)]
    pub fn pow(&self, other: Numberish, decimals: Option<u8>) -> Result<WasmFixedPoint, Error> {
        Ok(WasmFixedPoint {
            inner: self
                .inner
                .pow(WasmFixedPoint::new(other, decimals)?.inner)
                .to_result()?,
            decimals: self.decimals,
        })
    }

    /// Get the natural logarithm of this fixed-point number.
    #[wasm_bindgen(skip_jsdoc)]
    pub fn ln(&self) -> Result<WasmFixedPoint, Error> {
        Ok(WasmFixedPoint {
            inner: ln(self.inner.raw()).to_result()?.fixed(),
            decimals: self.decimals,
        })
    }

    /// Find out if this number is equal to another.
    #[wasm_bindgen(skip_jsdoc)]
    pub fn eq(&self, other: Numberish, decimals: Option<u8>) -> Result<bool, Error> {
        Ok(self.inner == WasmFixedPoint::new(other, decimals)?.inner)
    }

    /// Find out if this number is not equal to another.
    #[wasm_bindgen(skip_jsdoc)]
    pub fn ne(&self, other: Numberish, decimals: Option<u8>) -> Result<bool, Error> {
        Ok(self.inner != WasmFixedPoint::new(other, decimals)?.inner)
    }

    /// Find out if this number is greater than another.
    #[wasm_bindgen(skip_jsdoc)]
    pub fn gt(&self, other: Numberish, decimals: Option<u8>) -> Result<bool, Error> {
        Ok(self.inner > WasmFixedPoint::new(other, decimals)?.inner)
    }

    /// Find out if this number is greater than or equal to another.
    #[wasm_bindgen(skip_jsdoc)]
    pub fn gte(&self, other: Numberish, decimals: Option<u8>) -> Result<bool, Error> {
        Ok(self.inner >= WasmFixedPoint::new(other, decimals)?.inner)
    }

    /// Find out if this number is less than another.
    #[wasm_bindgen(skip_jsdoc)]
    pub fn lt(&self, other: Numberish, decimals: Option<u8>) -> Result<bool, Error> {
        Ok(self.inner < WasmFixedPoint::new(other, decimals)?.inner)
    }

    /// Find out if this number is less than or equal to another.
    #[wasm_bindgen(skip_jsdoc)]
    pub fn lte(&self, other: Numberish, decimals: Option<u8>) -> Result<bool, Error> {
        Ok(self.inner <= WasmFixedPoint::new(other, decimals)?.inner)
    }

    /// Get the minimum of this number and another.
    ///
    /// If the numbers are equal, the number with the fewest decimal places will be returned.
    #[wasm_bindgen(skip_jsdoc)]
    pub fn min(&self, other: Numberish, decimals: Option<u8>) -> Result<WasmFixedPoint, Error> {
        Ok(WasmFixedPoint {
            inner: self.inner.min(WasmFixedPoint::new(other, decimals)?.inner),
            decimals: self.decimals,
        })
    }

    /// Get the maximum of this number and another.
    ///
    /// If the numbers are equal, the number with the most decimal places will be returned.
    #[wasm_bindgen(skip_jsdoc)]
    pub fn max(&self, other: Numberish, decimals: Option<u8>) -> Result<WasmFixedPoint, Error> {
        Ok(WasmFixedPoint {
            inner: self.inner.max(WasmFixedPoint::new(other, decimals)?.inner),
            decimals: self.decimals,
        })
    }

    /// Clamp this number to a range.
    #[wasm_bindgen(skip_jsdoc)]
    pub fn clamp(
        &self,
        min: Numberish,
        max: Numberish,
        decimals: Option<u8>,
    ) -> Result<WasmFixedPoint, Error> {
        Ok(WasmFixedPoint {
            inner: self.inner.clamp(
                WasmFixedPoint::new(min, decimals)?.inner,
                WasmFixedPoint::new(max, decimals)?.inner,
            ),
            decimals: self.decimals,
        })
    }

    // Conversion //

    /// Create a new fixed-point number from this one, with a given number of
    /// decimal places.
    ///
    /// @example
    /// ```ts
    /// const a = fixed(1e18);
    /// console.log(a.toString());
    /// // => 1.000000000000000000
    ///
    /// const b = a.toFixed(6);
    /// console.log(b.toString());
    /// // => 1.000000
    /// ```
    #[wasm_bindgen(skip_jsdoc, js_name = toFixed)]
    pub fn to_fixed(&self, decimals: u8) -> WasmFixedPoint {
        WasmFixedPoint {
            inner: self.inner / WasmFixedPoint::scale_factor(decimals)
                * WasmFixedPoint::scale_factor(decimals),
            decimals,
        }
    }

    /// Get the float representation of this fixed-point number.
    ///
    /// __Caution__: This method may lose precision.
    ///
    /// @example
    ///
    /// ```ts
    /// const num = fixed(1_123456789012345678n);
    /// console.log(num.toNumber());
    /// // 1.1234567890123457
    /// ```
    #[wasm_bindgen(skip_jsdoc, js_name = toNumber)]
    pub fn to_number(&self) -> f64 {
        parse_float(&self.to_string())
    }

    /// Get the scaled hexadecimal string representation of this fixed-point
    /// number with the `0x` prefix.
    ///
    /// @example
    /// ```ts
    /// const num = fixed(1_123456789012345678n);
    /// console.log(num.toHex());
    /// // 0xf9751ff4d94f34e
    /// ```
    #[wasm_bindgen(js_name = toHex)]
    pub fn to_hex(&self) -> Result<String, Error> {
        let hex = self.bigint()?.to_string(16).map_err(|e| error!("{e:?}"))?;
        Ok(JsString::from("0x").concat(&hex).into())
    }

    /// Get the decimal string representation of this fixed-point number.
    #[wasm_bindgen(skip_jsdoc, js_name = toString)]
    pub fn to_string(&self) -> String {
        let str = self.inner.to_string();

        // If there are no decimals, return the integer part only.
        if self.decimals == 0 {
            return str.split('.').next().unwrap_or_default().to_string();
        }

        let decimals_delta = INNER_DECIMALS - self.decimals;
        if decimals_delta == 0 {
            return str;
        }
        str[..str.len() - decimals_delta as usize].to_string()
    }

    #[wasm_bindgen(skip_jsdoc, js_name = valueOf)]
    pub fn value_of(&self) -> String {
        self.to_string()
    }

    // Internal //

    fn scale_factor(decimals: u8) -> FixedPoint<I256> {
        let factor = I256::from(10).pow((INNER_DECIMALS * 2 - decimals) as u32);
        FixedPoint::from(factor)
    }
}

// Types //

#[wasm_bindgen(typescript_custom_section)]
const _: &'static str = r#"
/**
 * A number-like value that can be parsed into a {@linkcode FixedPoint} number.
 */
export type Numberish = FixedPoint | bigint | number | string;
"#;

#[wasm_bindgen]
extern "C" {
    /// `FixedPoint | bigint | number | string`
    #[derive(Clone)]
    #[wasm_bindgen(typescript_type = Numberish)]
    pub type Numberish;

    #[wasm_bindgen(js_name = valueOf, method)]
    fn value_of(this: &Numberish) -> Numberish;

    #[wasm_bindgen(js_name = toString, method)]
    fn to_string(this: &Numberish) -> JsString;

    #[wasm_bindgen(getter, method)]
    fn is_fixed_point(this: &Numberish) -> Option<bool>;
}

impl TryFrom<Numberish> for FixedPoint<I256> {
    type Error = Error;

    #[track_caller]
    fn try_from(num: Numberish) -> Result<Self, Self::Error> {
        if num.is_fixed_point() == Some(true) {
            let s = format!(
                "{}e{}",
                num.to_string().as_string().unwrap_or_default(),
                INNER_DECIMALS
            );
            return FixedPoint::from_dec_str(&s).to_result();
        }

        match num.js_typeof().as_string().unwrap_or_default().as_str() {
            "bigint" | "number" => Ok(num.to_i256()?.fixed()),
            _ => {
                let mut s = num
                    .to_string()
                    .trim()
                    .to_lower_case()
                    .replace_all("_", "")
                    .replace_all(",", "");
                let mut sign = 1;
                if s.starts_with("-", 0) {
                    s = s.slice(1, s.length());
                    sign = -1;
                };
                if s.starts_with("+", 0) {
                    s = s.slice(1, s.length());
                };
                if s.starts_with("0x", 0) || s.starts_with("0b", 0) || s.starts_with("0o", 0) {
                    let int = s.to_i256()? * I256::from(sign);
                    return Ok(int.fixed());
                };
                let s = s.as_string().unwrap_or_default();
                let int = i256_from_str(&s).to_result()? * I256::from(sign);
                Ok(int.fixed())
            }
        }
    }
}

#[ts]
struct GenerateRandomParams {
    /// The minimum value to generate.
    ///
    /// @default 0
    min: Option<Numberish>,

    /// The maximum value to generate.
    ///
    /// @default min + parseFixed(1.0, decimals)
    max: Option<Numberish>,

    /// The number of decimal places to use. Max is `18`.
    ///
    /// @default 18
    decimals: Option<u8>,
}
