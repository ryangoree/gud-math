# @gud/math

[![GitHub](https://img.shields.io/badge/ryangoree%2Fgud--math-151b23?logo=github)](https://github.com/ryangoree/gud-math)
[![NPM Version](https://img.shields.io/badge/%40gud%2Fmath-cb3837?logo=npm)](https://npmjs.com/package/@gud/math)
[![License: Apache-2.0](https://img.shields.io/badge/Apache%202.0-23454d?logo=apache)](./LICENSE)

**Effortless fixed-point math in TypeScript powered by WASM.**

## Installing

```sh
npm install @gud/math
# or
yarn add @gud/math
```

## Usage

```ts
import { fixed, parseFixed } from "@gud/math";

const principal = fixed(1_000_123456789012345678n);;
const rate = parseFixed(0.025); // parse unscaled values

const interest = principal.mulDown(rate);
const total = principal.add(interest);

console.log(total.bigint);
// => 1025126543208737654319n

console.log(total.toNumber());
// => 1025.1265432087378

console.log(total.toString()); // or console.log(`${total}`)
// => "1025.126543208737654319"

console.log(total.format({ decimals: 4 }))
// => "1,025.1265"

console.log(total.formatCurrency())
// => "$1,025.13"
```

## Custom decimal places

By default, all fixed-point numbers are created with 18 decimal places, but this can be changed by passing a second argument to the `fixed` and `parseFixed` functions.

```ts
const n = fixed(1500000n, 6);
console.log(n.bigint, n.toString());
// => 1500000n "1.500000"

const n2 = parseFixed(0.025, 6);
console.log(n2.bigint, n2.toString());
// => 25000n "0.025000"
```

## `fixed` function

Use `fixed` with values that are already scaled.

```ts
const totalSupply = 22950342684077248430458n;
const sharePrice = 1094205545459194143n;
const totalSupplyInBase = fixed(totalSupply).mulDown(sharePrice);
console.log(`Total supply in base: ${totalSupplyInBase.bigint}`);
// => "Total supply in base: 25112392235106171381320"
```

```ts
const fromBigint = fixed(1500000000000000000n);
const fromNumber = fixed(1.5e18);
const fromString = fixed("1.5e18");
const withDecimals = fixed(1.5e6, 6);

console.log(fromBigint.toString());
// => "1.500000000000000000"

console.log(fromNumber.toString());
// => "1.500000000000000000"

console.log(fromString.toString());
// => "1.500000000000000000"

console.log(withDecimals.toString());
// => "1.500000"
```

## `parseFixed` function

Use `parseFixed` with unscaled decimal values.

```ts
const principal = "1_000.123456789012345678";
const rate = 0.025;
const interest = parseFixed(principal).mulUp(parseFixed(rate));

console.log(`Interest: ${interest}`);
// => "Interest: 25.003086419725308642"
```

```ts
const fromNumber = parseFixed(1.5);
const fromString = parseFixed("1.5");
const withDecimals = parseFixed("1.5", 6);

console.log(fromNumber.toString());
// => "1.500000000000000000"

console.log(fromString.toString());
// => "1.500000000000000000"

console.log(withDecimals.toString());
// => "1.500000"
```

## `randomFixed` function

Use `randomFixed` to generate a random fixed-point number.

```ts
let rand = randomFixed({
  min: 1e18,
  max: 100e18,
});
console.log(`Random: ${rand}`);
// => "Random: 69.357623681464768420"
```
