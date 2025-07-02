import { parseFixed, randomFixed } from '@gud/math';

const principal = randomFixed({ min: '100e18', max: '100_000e18' });
const rate = parseFixed(0.025);

const interest = principal.mulDown(rate);
const total = principal.add(interest);

console.table({
  'Scaled Int': total.bigint,
  Number: total.toNumber(),
  String: total.toString(), // or console.log(`${total}`)
  'Formatted String': total.format({ decimals: 4 }),
  'Currency String': total.formatCurrency(),
});
