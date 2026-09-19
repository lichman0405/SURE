// The check SURE would carry out for this project, if it were allowed to and if
// there were a container to carry it out in.
//
// It is a check and not a demonstration: it sums the monthly totals and fails
// loudly if a row is not a number. Nothing here is installed and nothing here is
// fetched — it uses the language's own console and nothing else.
const rows = [
  { month: '2026-01', amount: 1200 },
  { month: '2026-02', amount: 980 },
];

const notNumbers = rows.filter((row) => typeof row.amount !== 'number');
if (notNumbers.length > 0) {
  console.error(`${notNumbers.length} row(s) are not numbers`);
  process.exit(1);
}

console.log(`totals: ${rows.reduce((sum, row) => sum + row.amount, 0)}`);
