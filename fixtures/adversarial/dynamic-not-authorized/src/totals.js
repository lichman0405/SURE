// The monthly totals a billing screen shows.
function totals(lines) {
  return lines.reduce((sum, line) => sum + line.amount, 0);
}

module.exports = { totals };
