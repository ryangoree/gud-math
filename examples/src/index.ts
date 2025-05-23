import { parseFixed, randomFixed } from '@rygo/mantissa';

const principal = randomFixed({ min: '100e18', max: '100_000e18' });
const rate = parseFixed(0.025);

const interest = principal.mulDown(rate);
const total = principal.add(interest);

console.log(total.bigint);
// => 1025126543208737654319n

console.log(total.toNumber());
// => 1025.1265432087378

console.log(total.toString()); // or console.log(`${total}`)
// => "1025.126543208737654319"

console.log(total.format({ decimals: 4 }));
// => "1,025.1265"

console.log(total.formatCurrency());
// => "$1,025.13"
