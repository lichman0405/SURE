const express = require('express');
const app = express();

app.get('/invoices', listInvoices);

module.exports = app;
