const express = require('express');
const app = express();

app.get('/bookings', listBookings);

module.exports = app;
