use utils_core::error::Error;
use wasm_bindgen::prelude::*;

use crate::{IGenerateRandomParams, Numberish, WasmFixedPoint};

/// Get the version of this package.
#[wasm_bindgen(skip_jsdoc, js_name = getVersion)]
pub fn get_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Create a new fixed-point number from a scaled value or another fixed-point number. If the value
/// is already a fixed-point number instance, the number of decimal places will be adjusted to match
/// the new value.
///
/// @param value - A scaled value between `-2^255` and `2^255 - 1` (signed 256-bit integer, i.e.,
/// `int256`).
///
/// @param decimals - The number of decimal places to use. Max is `18`. Defaults to `18`.
///
/// @example
/// ```js
/// const fromBigint = fixed(1500000000000000000n);
/// const fromNumber = fixed(1.5e18);
/// const fromString = fixed('1.5e18');
/// const withDecimals = fixed(1.5e6, 6);
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
#[wasm_bindgen(skip_jsdoc)]
pub fn fixed(value: Numberish, decimals: Option<u8>) -> Result<WasmFixedPoint, Error> {
    WasmFixedPoint::new(value, decimals)
}

/// Create a fixed-point number by parsing a decimal value and scaling it by a given number of
/// decimal places.
///
/// @param value - A value to parse and scale.
///
/// @param decimals - The number of decimal places to use. Max is `18`. Defaults to `18`.
///
/// @example
//
/// ```js
/// const fromNumber = parseFixed(1.5);
/// const fromString = parseFixed('1.5');
/// const withDecimals = parseFixed('1.5', 6);
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
#[wasm_bindgen(skip_jsdoc, js_name = parseFixed)]
pub fn parse_fixed(value: Numberish, decimals: Option<u8>) -> Result<WasmFixedPoint, Error> {
    WasmFixedPoint::parse(value, decimals)
}

/// Create a random fixed-point number with an optional min and max.
///
/// @example
///
/// ```ts
/// const random = randomFixed();
/// const randomWithOptions = randomFixed({
///   min: 0.5,
///   max: 100,
///   decimals: 6,
/// });
///
/// console.log(random.toString());
/// // => 0.472987274007185487
///
/// console.log(randomWithOptions.toString());
/// // => 0.500000
/// ```
#[wasm_bindgen(skip_jsdoc, js_name = randomFixed)]
pub fn random_fixed(params: Option<IGenerateRandomParams>) -> Result<WasmFixedPoint, Error> {
    WasmFixedPoint::random(params)
}
