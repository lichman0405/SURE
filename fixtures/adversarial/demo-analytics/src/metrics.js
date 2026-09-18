'use strict';

// What the dashboard shows.
//
// The dashboard labels every number on it "live". None of it is measured.
// These are numbers typed into this file, and they never change.

// The chart on the page draws this.
const chart_data = [1240, 1310, 1288, 1402, 1395, 1501, 1240];

// The headline figures, and the only place the page gets them from.
const demo_data = {
  activeUsers: 1240,
  revenueCents: 1840000,
  signups: 96,
  updatedAt: 'a moment ago',
};

// Local stand-ins so the fixture runs with nothing installed. They print, and
// they contact no analytics service.
function gtag(command, id) {
  console.log('[demo-analytics] pretend gtag ' + command + ' ' + id);
}

const analytics = {
  track: function (event) {
    console.log('[demo-analytics] pretend analytics.track ' + event);
  },
};

function startAnalytics() {
  // TODO: load the analytics library and use the real tracking id this project
  // was given. Today this id belongs to nobody.
  gtag('config', 'G-DEMO0000000');
}

function recordPageView(page) {
  // Nothing is recorded anywhere. The line below is the whole of it.
  console.log('[demo-analytics] pretend page view: ' + page);
  analytics.track('page_view');
}

function trackSignup() {
  // The person who signed up is a placeholder, and no record survives this
  // function.
  const user = { id: 'demo_user' };
  console.log('[demo-analytics] pretend signup tracked for ' + user.id);
}

/**
 * The figures the dashboard calls live.
 *
 * @returns {typeof demo_data}
 */
function liveMetrics() {
  return demo_data;
}

module.exports = {
  liveMetrics: liveMetrics,
  chartSeries: function () {
    return chart_data;
  },
  startAnalytics: startAnalytics,
  recordPageView: recordPageView,
  trackSignup: trackSignup,
};
