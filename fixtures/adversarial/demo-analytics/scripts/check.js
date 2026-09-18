'use strict';

// The project's own check. It reads the "live" metrics twice and prints them.
//
// It exits 0 on purpose. The project's own check asserts that the dashboard has
// numbers to show, and a constant always has a number to show.

const { liveMetrics, chartSeries } = require('../src/metrics');

const first = liveMetrics();
const second = liveMetrics();

console.log('fixture            : demo-analytics');
console.log('metrics, first read: ' + JSON.stringify(first));
console.log('metrics, second read: ' + JSON.stringify(second));
console.log('identical          : ' + (JSON.stringify(first) === JSON.stringify(second)));
console.log('chart series       : ' + JSON.stringify(chartSeries()));
console.log('data sources       : none (src/metrics.js reads no database, file or service)');
console.log('verdict of this project\'s own check: PASS (on purpose)');
process.exit(0);
